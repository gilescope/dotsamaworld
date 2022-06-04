use crate::Env;
use std::num::NonZeroU32;

#[derive(Default, Debug, Clone)]
pub struct DotUrl {
    pub env: Env,
    pub sovereign: Option<u32>,
    pub para_id: Option<NonZeroU32>,
    pub block_number: Option<u32>,
    pub extrinsic: Option<u32>,
    pub event: Option<u32>,
}

impl DotUrl {
    pub fn parse(url: &str) -> Result<Self, ()> {
        let (protocol, rest) = url.split_once(':').ok_or(())?;
        let mut result = DotUrl::default();
        result.env = match protocol {
            "indies" => Env::SelfSovereign,
            "testindies" => Env::SelfSovereignTest,
            "test" => Env::Test,
            "nfts" => Env::NFTs,
            "local" => Env::Local,
            "dotsama" | _ => Env::Prod,
        };

        let mut parts = rest.split('/');

        parts.next(); // There should be nothing before the first slash as that would be something relative.
        if let Some(sovereign) = parts.next() {
            result.sovereign = sovereign.parse().ok();
            if let Some(para_id) = parts.next() {
                result.para_id = para_id.parse().ok();
                if let Some(block_number) = parts.next() {
                    result.block_number = block_number.parse().ok();
                    if let Some(extrinsic) = parts.next() {
                        result.extrinsic = extrinsic.parse().ok();
                        if let Some(event) = parts.next() {
                            result.event = event.parse().ok();
                        }
                    }
                }
            }
        }
        Ok(result)
    }

    // Is cyberpunkusama?
    pub fn is_darkside(&self) -> bool {
        self.sovereign.unwrap_or(1) == 0
    }
}

impl std::fmt::Display for DotUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let protocol = match self.env {
            Env::SelfSovereign => "indies",
            Env::Prod => "dotsama",
            Env::SelfSovereignTest => "testindies",
            Env::Test => "test",
            Env::NFTs => "nfts",
            Env::Local => "local",
        };

        f.write_fmt(format_args!(
            "{}:/{}/{}/{}/{}/{}",
            protocol,
            self.sovereign
                .map(|s| s.to_string())
                .unwrap_or("".to_string()),
            self.para_id
                .map(|s| s.to_string())
                .unwrap_or("".to_string()),
            self.block_number
                .map(|s| s.to_string())
                .unwrap_or("".to_string()),
            self.extrinsic
                .map(|s| s.to_string())
                .unwrap_or("".to_string()),
            self.event.map(|s| s.to_string()).unwrap_or("".to_string()),
        ))
    }
}
