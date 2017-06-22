#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub pretty: bool,
    pub indent: u32,
    pub single: bool,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct EmptyConfig {}

impl Default for Config {
    fn default() -> Config {
        Config {
            indent: 2,
            pretty: false,
            single: false,
        }
    }
}
