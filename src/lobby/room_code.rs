use rand::Rng;

const WORDS: &[&str] = &[
    "ABLE", "ALSO", "AREA", "AWAY", "BACK", "BALL", "BASE", "BEAR", "BEAT", "BEEN", "BEST", "BIRD",
    "BLUE", "BOAT", "BODY", "BOOK", "BOTH", "CALL", "CAME", "CARE", "CASE", "CITY", "COME", "COOL",
    "DARK", "DATA", "DAWN", "DAYS", "DEAL", "DEEP", "DONE", "DOOR", "DOWN", "DRAW", "DREW", "EACH",
    "EAST", "EASY", "EDGE", "EVEN", "EVER", "FACE", "FACT", "FAIR", "FALL", "FARM", "FAST", "FEEL",
    "FEET", "FELL", "FELT", "FILE", "FILL", "FIND", "FINE", "FIRE", "FISH", "FIVE", "FLAT", "FLOW",
    "FOLK", "FOOD", "FOOT", "FORM", "FOUR", "FREE", "FROM", "FULL", "GAME", "GAVE", "GIRL", "GIVE",
    "GLAD", "GOAL", "GOES", "GOLD", "GONE", "GOOD", "GRAY", "GREW", "HALF", "HAND", "HARD", "HEAD",
    "HEAR", "HEAT", "HELD", "HELP", "HERE", "HIGH", "HOLD", "HOME", "HOPE", "HOUR", "HUGE", "IDEA",
    "INTO", "IRON", "ITEM", "JUST", "KEEP", "KEPT", "KIND", "KING", "KNEW", "KNOW", "LAND", "LAST",
    "LATE", "LEAD", "LEFT", "LESS", "LIFE", "LIKE", "LINE", "LIST", "LIVE", "LONG", "LOOK", "LORD",
    "LOSE", "LOST", "LOVE", "MADE", "MAIN", "MAKE", "MANY", "MARK", "MASS", "MEAN", "MEET", "MILE",
    "MIND", "MINE", "MISS", "MODE", "MORE", "MOST", "MOVE", "MUCH", "MUST", "NAME", "NEAR", "NEED",
    "NEXT", "NICE", "NODE", "NOTE", "ONCE", "ONLY", "OPEN", "OVER", "PAGE", "PAID", "PAIR", "PARK",
    "PART", "PASS", "PAST", "PATH", "PLAN", "PLAY", "PLUS", "POET", "POLL", "POOL", "POOR", "PORT",
    "POST", "PULL", "PURE", "PUSH", "RACE", "RAIN", "RANG", "RANK", "RATE", "READ", "REAL", "REAR",
    "RELY", "REST", "RICE", "RICH", "RIDE", "RING", "RISE", "ROAD", "ROCK", "ROLE", "ROLL", "ROOF",
    "ROOM", "ROOT", "ROSE", "RULE", "SAFE", "SAID", "SALE", "SAME", "SAND", "SAVE", "SEAT", "SEEM",
    "SEEN", "SELF", "SELL", "SEND", "SENT", "SHIP", "SHOP", "SHOT", "SHOW", "SHUT", "SIDE", "SIGN",
    "SITE", "SIZE", "SLOW", "SNOW", "SOFT", "SOIL", "SOLD", "SOME", "SONG", "SOON", "SORT", "SOUL",
    "SPOT", "STAR", "STAY", "STEP", "STOP", "SUCH", "SUIT", "SURE", "TAKE", "TALE", "TALK", "TALL",
    "TASK", "TEAM", "TELL", "TERM", "TEST", "TEXT", "THAN", "THAT", "THEM", "THEN", "THEY", "THIS",
    "THUS", "TIME", "TINY", "TOLD", "TONE", "TOOK", "TOOL", "TOUR", "TOWN", "TREE", "TRUE", "TUNE",
    "TURN", "TYPE", "UNIT", "UPON", "USED", "USER", "VARY", "VAST", "VERY", "VIEW", "VOTE", "WAGE",
    "WAIT", "WALK", "WALL", "WANT", "WARD", "WARM", "WAVE", "WAYS", "WEAK", "WEAR", "WEEK", "WELL",
    "WENT", "WERE", "WEST", "WHAT", "WHEN", "WHOM", "WIDE", "WIFE", "WILD", "WILL", "WIND", "WINE",
    "WING", "WIRE", "WISE", "WISH", "WITH", "WOOD", "WORD", "WORE", "WORK", "WORN", "YARD", "YEAH",
    "YEAR", "YOUR", "ZONE",
];

pub fn generate() -> String {
    let mut rng = rand::thread_rng();
    let word = WORDS[rng.gen_range(0..WORDS.len())];
    word.to_string()
}
