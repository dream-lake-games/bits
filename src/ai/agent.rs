use async_openai::config::Config;
use async_openai::types::responses::{
    CreateResponseArgs, FunctionCallOutput, FunctionCallOutputItemParam, FunctionTool, InputItem,
    InputParam, Item, OutputItem, ResponseFormatJsonSchema, ResponseTextParam,
    TextResponseFormatConfiguration, Tool as OpenAITool,
};
use async_openai::Client;
use bevy::log::{debug, trace, warn};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

trait ToolHandler: Send + Sync {
    fn call(&self, args: String) -> BoxFuture<'_, anyhow::Result<String>>;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
}

struct ToolImpl<F, Args, Ret> {
    name: String,
    description: String,
    parameters_schema: serde_json::Value,
    handler: F,
    _phantom: std::marker::PhantomData<fn(Args) -> Ret>,
}

impl<F, Args, Ret> ToolHandler for ToolImpl<F, Args, Ret>
where
    F: Fn(Args) -> BoxFuture<'static, anyhow::Result<Ret>> + Send + Sync + 'static,
    Args: DeserializeOwned + Send + 'static,
    Ret: Serialize + Send + 'static,
{
    fn call(&self, args: String) -> BoxFuture<'_, anyhow::Result<String>> {
        Box::pin(async move {
            let parsed: Args = serde_json::from_str(&args)?;
            let result = (self.handler)(parsed).await?;
            Ok(serde_json::to_string(&result)?)
        })
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.parameters_schema.clone()
    }
}

/// A typed tool that can be added to an agent. The argument type must implement
/// `JsonSchema` for automatic schema generation.
pub struct Tool<Args, F> {
    name: &'static str,
    description: &'static str,
    handler: F,
    _args: std::marker::PhantomData<Args>,
}

impl<Args, F, Ret> Tool<Args, F>
where
    Args: DeserializeOwned + JsonSchema + Send + 'static,
    F: Fn(Args) -> BoxFuture<'static, anyhow::Result<Ret>> + Send + Sync + 'static,
    Ret: Serialize + Send + 'static,
{
    pub fn new(name: &'static str, description: &'static str, handler: F) -> Self {
        Self {
            name,
            description,
            handler,
            _args: std::marker::PhantomData,
        }
    }

    fn into_handler(self) -> Arc<dyn ToolHandler> {
        let schema = schemars::schema_for!(Args);
        let mut schema_value = serde_json::to_value(schema).unwrap_or_default();
        make_schema_strict(&mut schema_value);

        Arc::new(ToolImpl {
            name: self.name.to_string(),
            description: self.description.to_string(),
            parameters_schema: schema_value,
            handler: self.handler,
            _phantom: std::marker::PhantomData,
        })
    }
}

#[derive(Debug)]
pub struct AgentOutput<T> {
    pub output: T,
    pub tool_calls: usize,
}

/// Transforms a schemars-generated JSON schema to be compatible with OpenAI's strict mode.
fn make_schema_strict(schema: &mut serde_json::Value) {
    if let Some(obj) = schema.as_object_mut() {
        if let Some(any_of) = obj.remove("anyOf") {
            if let Some(any_of_arr) = any_of.as_array() {
                let mut types = Vec::new();
                let mut inner_schema = None;
                for item in any_of_arr {
                    if let Some(t) = item.get("type").and_then(|t| t.as_str()) {
                        if t == "null" {
                            types.push("null");
                        } else {
                            types.push(t);
                            inner_schema = Some(item.clone());
                        }
                    }
                }
                if !types.is_empty() {
                    obj.insert("type".to_string(), serde_json::json!(types));
                    if let Some(inner) = inner_schema {
                        if let Some(inner_obj) = inner.as_object() {
                            for (k, v) in inner_obj {
                                if k != "type" && !obj.contains_key(k) {
                                    obj.insert(k.clone(), v.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        if obj.get("type").and_then(|t| t.as_str()) == Some("object") {
            obj.insert(
                "additionalProperties".to_string(),
                serde_json::json!(false),
            );
            if let Some(props) = obj.get("properties") {
                if let Some(props_obj) = props.as_object() {
                    let all_keys: Vec<String> = props_obj.keys().cloned().collect();
                    obj.insert("required".to_string(), serde_json::json!(all_keys));
                }
            }
        }
        if let Some(props) = obj.get_mut("properties") {
            if let Some(props_obj) = props.as_object_mut() {
                for (_, prop_schema) in props_obj.iter_mut() {
                    make_schema_strict(prop_schema);
                }
            }
        }
        if let Some(items) = obj.get_mut("items") {
            make_schema_strict(items);
        }
    }
}

fn build_output_schema<Output: JsonSchema>() -> serde_json::Value {
    let output_schema = schemars::schema_for!(Output);
    let mut schema_value = serde_json::to_value(output_schema).unwrap_or_default();
    make_schema_strict(&mut schema_value);
    schema_value
}

/// An AI agent that can use tools and produce typed, structured outputs.
pub struct Agent {
    name: &'static str,
    system_prompt: String,
    tools: Vec<Arc<dyn ToolHandler>>,
    max_turns: usize,
}

impl Agent {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            system_prompt: String::new(),
            tools: vec![],
            max_turns: 10,
        }
    }

    pub fn system(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    pub fn tool<Args, F, Ret>(mut self, tool: Tool<Args, F>) -> Self
    where
        Args: DeserializeOwned + JsonSchema + Send + 'static,
        F: Fn(Args) -> BoxFuture<'static, anyhow::Result<Ret>> + Send + Sync + 'static,
        Ret: Serialize + Send + 'static,
    {
        self.tools.push(tool.into_handler());
        self
    }

    pub fn max_turns(mut self, n: usize) -> Self {
        self.max_turns = n;
        self
    }

    /// Run the agent with the given input and return a typed output.
    pub async fn run<Output, C>(
        &self,
        client: &Client<C>,
        input: &str,
    ) -> anyhow::Result<AgentOutput<Output>>
    where
        Output: DeserializeOwned + JsonSchema,
        C: Config,
    {
        debug!("[Agent:{}] Starting (input_len={})", self.name, input.len());
        trace!("[Agent:{}] Full input:\n{}", self.name, input);

        let output_schema = build_output_schema::<Output>();
        let openai_tools: Vec<OpenAITool> = self
            .tools
            .iter()
            .map(|t| {
                OpenAITool::Function(FunctionTool {
                    name: t.name().to_string(),
                    description: Some(t.description().to_string()),
                    parameters: Some(t.parameters_schema()),
                    strict: Some(true),
                })
            })
            .collect();

        let mut input_items: Vec<InputItem> = vec![];
        let initial_input = input.to_string();
        let mut total_tool_calls = 0;

        for turn in 0..self.max_turns {
            trace!("[Agent:{}] Turn {}/{}", self.name, turn + 1, self.max_turns);

            let text_config = ResponseTextParam {
                format: TextResponseFormatConfiguration::JsonSchema(ResponseFormatJsonSchema {
                    name: format!("{}_output", self.name),
                    description: Some("Final structured output".to_string()),
                    schema: Some(output_schema.clone()),
                    strict: Some(true),
                }),
                verbosity: None,
            };

            let input_param = if input_items.is_empty() {
                InputParam::Text(initial_input.clone())
            } else {
                InputParam::Items(input_items.clone())
            };

            let request = if openai_tools.is_empty() {
                CreateResponseArgs::default()
                    .model("gpt-4.1-mini")
                    .instructions(&self.system_prompt)
                    .input(input_param)
                    .text(text_config)
                    .build()?
            } else {
                CreateResponseArgs::default()
                    .model("gpt-4.1-mini")
                    .instructions(&self.system_prompt)
                    .input(input_param)
                    .text(text_config)
                    .tools(openai_tools.clone())
                    .build()?
            };
            let response = client.responses().create(request).await?;

            let function_calls: Vec<_> = response
                .output
                .iter()
                .filter_map(|item| {
                    if let OutputItem::FunctionCall(call) = item {
                        Some(call)
                    } else {
                        None
                    }
                })
                .collect();

            if !function_calls.is_empty() {
                debug!(
                    "[Agent:{}] {} tool call(s)",
                    self.name,
                    function_calls.len()
                );

                if input_items.is_empty() {
                    input_items.push(InputItem::EasyMessage(
                        async_openai::types::responses::EasyInputMessage {
                            r#type: async_openai::types::responses::MessageType::Message,
                            role: async_openai::types::responses::Role::User,
                            content: async_openai::types::responses::EasyInputContent::Text(
                                initial_input.clone(),
                            ),
                        },
                    ));
                }

                for call in function_calls {
                    trace!(
                        "[Agent:{}] Tool '{}' called with: {}",
                        self.name, call.name, call.arguments
                    );

                    input_items.push(InputItem::Item(Item::FunctionCall(call.clone())));

                    let result = if let Some(handler) =
                        self.tools.iter().find(|t| t.name() == call.name)
                    {
                        match handler.call(call.arguments.clone()).await {
                            Ok(output) => output,
                            Err(e) => format!("Error: {}", e),
                        }
                    } else {
                        format!("Unknown tool: {}", call.name)
                    };

                    trace!("[Agent:{}] Tool '{}' result: {}", self.name, call.name, result);

                    input_items.push(InputItem::Item(Item::FunctionCallOutput(
                        FunctionCallOutputItemParam {
                            call_id: call.call_id.clone(),
                            output: FunctionCallOutput::Text(result),
                            id: None,
                            status: None,
                        },
                    )));
                    total_tool_calls += 1;
                }
                continue;
            }

            if let Some(text) = response.output_text() {
                trace!("[Agent:{}] Raw response: {}", self.name, text);

                match serde_json::from_str::<Output>(&text) {
                    Ok(output) => {
                        debug!(
                            "[Agent:{}] ✓ Completed ({} tool call{})",
                            self.name,
                            total_tool_calls,
                            if total_tool_calls == 1 { "" } else { "s" }
                        );
                        return Ok(AgentOutput {
                            output,
                            tool_calls: total_tool_calls,
                        });
                    }
                    Err(e) => {
                        warn!("[Agent:{}] Parse error: {}", self.name, e);
                        trace!("[Agent:{}] Failed to parse: {}", self.name, text);
                        if input_items.is_empty() {
                            input_items.push(InputItem::EasyMessage(
                                async_openai::types::responses::EasyInputMessage {
                                    r#type: async_openai::types::responses::MessageType::Message,
                                    role: async_openai::types::responses::Role::User,
                                    content: async_openai::types::responses::EasyInputContent::Text(
                                        initial_input.clone(),
                                    ),
                                },
                            ));
                        }
                        input_items.push(InputItem::EasyMessage(
                            async_openai::types::responses::EasyInputMessage {
                                r#type: async_openai::types::responses::MessageType::Message,
                                role: async_openai::types::responses::Role::User,
                                content: async_openai::types::responses::EasyInputContent::Text(
                                    format!("Error parsing output: {}. Please try again.", e),
                                ),
                            },
                        ));
                    }
                }
            } else {
                trace!("[Agent:{}] No text output", self.name);
            }
        }

        anyhow::bail!(
            "Agent '{}' did not produce output after {} turns",
            self.name,
            self.max_turns
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, serde::Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct SimpleOutput {
        message: String,
    }

    #[derive(Debug, serde::Deserialize, JsonSchema)]
    struct SearchArgs {
        query: String,
    }

    fn mock_search_tool(
    ) -> Tool<SearchArgs, impl Fn(SearchArgs) -> BoxFuture<'static, anyhow::Result<String>> + Send + Sync + 'static>
    {
        Tool::new(
            "search",
            "Search for information",
            |args: SearchArgs| -> BoxFuture<'static, anyhow::Result<String>> {
                Box::pin(async move { Ok(format!("Results for: {}", args.query)) })
            },
        )
    }

    #[test]
    fn test_tool_schema_generation() {
        let handler = mock_search_tool().into_handler();
        assert_eq!(handler.name(), "search");
        let schema = handler.parameters_schema();
        println!(
            "Tool schema: {}",
            serde_json::to_string_pretty(&schema).unwrap()
        );
        assert!(schema.get("properties").is_some());
    }

    #[test]
    fn test_agent_builder() {
        let agent = Agent::new("test")
            .system("You are helpful")
            .tool(mock_search_tool())
            .max_turns(5);

        assert_eq!(agent.name, "test");
        assert_eq!(agent.tools.len(), 1);
        assert_eq!(agent.max_turns, 5);
    }

    #[test]
    fn test_output_schema() {
        let schema = build_output_schema::<SimpleOutput>();
        println!(
            "Output schema: {}",
            serde_json::to_string_pretty(&schema).unwrap()
        );
        assert!(schema.get("properties").is_some());
    }
}
