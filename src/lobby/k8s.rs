use anyhow::Result;
use k8s_openapi::api::{batch::v1::Job, core::v1::Service};
use kube::{Api, Client, Config};
use std::collections::BTreeMap;
use std::env;

pub type K8sClient = Client;

fn server_image() -> String {
    env::var("BITS_SERVER_IMAGE").unwrap_or_else(|_| "bits-server:latest".to_string())
}

pub async fn create_client() -> Result<K8sClient> {
    let config = Config::infer().await?;
    let client = Client::try_from(config)?;
    Ok(client)
}

pub async fn create_game_server_job(
    client: &K8sClient,
    namespace: &str,
    room_code: &str,
    lobby_url: &str,
) -> Result<()> {
    use k8s_openapi::api::batch::v1::JobSpec;
    use k8s_openapi::api::core::v1::{Container, ContainerPort, PodSpec, PodTemplateSpec};
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

    let job_name = format!("server-{}", room_code.to_lowercase());
    let image = server_image();
    tracing::info!("Creating game server job {} with image {}", job_name, image);

    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), "bits-server".to_string());
    labels.insert("room".to_string(), room_code.to_lowercase());

    let container = Container {
        name: "server".to_string(),
        image: Some(image),
        image_pull_policy: Some("Never".to_string()),
        command: Some(vec!["/usr/local/bin/server".to_string()]),
        args: Some(vec![
            "--room-code".to_string(),
            room_code.to_string(),
            "--lobby-url".to_string(),
            lobby_url.to_string(),
        ]),
        ports: Some(vec![ContainerPort {
            container_port: 9000,
            protocol: Some("UDP".to_string()),
            ..Default::default()
        }]),
        ..Default::default()
    };

    let pod_spec = PodSpec {
        restart_policy: Some("Never".to_string()),
        containers: vec![container],
        ..Default::default()
    };

    let pod_template = PodTemplateSpec {
        metadata: Some(ObjectMeta {
            labels: Some(labels.clone()),
            ..Default::default()
        }),
        spec: Some(pod_spec),
    };

    let job_spec = JobSpec {
        ttl_seconds_after_finished: Some(60),
        template: pod_template,
        ..Default::default()
    };

    let job = Job {
        metadata: ObjectMeta {
            name: Some(job_name.clone()),
            labels: Some(labels),
            ..Default::default()
        },
        spec: Some(job_spec),
        ..Default::default()
    };

    let jobs: Api<Job> = Api::namespaced(client.clone(), namespace);
    jobs.create(&Default::default(), &job).await?;

    tracing::info!("Created job: {}", job_name);
    Ok(())
}

pub async fn create_loadbalancer_service(
    client: &K8sClient,
    namespace: &str,
    room_code: &str,
) -> Result<()> {
    use k8s_openapi::api::core::v1::{ServicePort, ServiceSpec};
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
    use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;

    let service_name = format!("server-{}", room_code.to_lowercase());

    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), "bits-server".to_string());
    labels.insert("room".to_string(), room_code.to_lowercase());

    let mut selector = BTreeMap::new();
    selector.insert("room".to_string(), room_code.to_lowercase());

    let service_spec = ServiceSpec {
        type_: Some("LoadBalancer".to_string()),
        selector: Some(selector),
        ports: Some(vec![ServicePort {
            port: 9000,
            target_port: Some(IntOrString::Int(9000)),
            protocol: Some("UDP".to_string()),
            ..Default::default()
        }]),
        ..Default::default()
    };

    let service = Service {
        metadata: ObjectMeta {
            name: Some(service_name.clone()),
            labels: Some(labels),
            ..Default::default()
        },
        spec: Some(service_spec),
        ..Default::default()
    };

    let services: Api<Service> = Api::namespaced(client.clone(), namespace);
    services.create(&Default::default(), &service).await?;

    tracing::info!("Created service: {}", service_name);
    Ok(())
}

pub async fn get_service_address(
    client: &K8sClient,
    namespace: &str,
    room_code: &str,
) -> Result<Option<String>> {
    let service_name = format!("server-{}", room_code.to_lowercase());
    let services: Api<Service> = Api::namespaced(client.clone(), namespace);

    let service = services.get(&service_name).await?;

    if let Some(status) = service.status {
        if let Some(load_balancer) = status.load_balancer {
            if let Some(ingress_list) = load_balancer.ingress {
                if let Some(ingress) = ingress_list.first() {
                    if let Some(ip) = &ingress.ip {
                        return Ok(Some(format!("{}:9000", ip)));
                    }
                    if let Some(hostname) = &ingress.hostname {
                        return Ok(Some(format!("{}:9000", hostname)));
                    }
                }
            }
        }
    }

    Ok(None)
}
