use std::env;
use std::fs::File;
use std::io::BufReader;

use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use serde::{self, Deserialize};

#[derive(Debug, Deserialize)]
struct Config {
    transforms: Vec<Transform>,
    silence_threshold_min: i64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum Transform {
    ReplaceAll(TransformReplaceAll),
    Regex(TransformRegex),
}

#[derive(Debug, Deserialize)]
struct TransformReplaceAll {
    from: String,
    to: String,
}

#[derive(Debug, Deserialize)]
struct TransformRegex {
    #[serde(with = "parse_regexp")]
    from: Regex,
    to: String,
}

mod parse_regexp {
    use regex::Regex;
    use serde::{self, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Regex, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Regex::new(s.as_str()).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Deserialize)]
struct DiscordChatExporterJson {
    messages: Vec<Message>,
}

#[derive(Debug, Deserialize)]
struct Message {
    #[serde(with = "parse_datetime")]
    timestamp: DateTime<Utc>,
    content: String,
    author: Author,
}

#[derive(Debug, Deserialize)]
struct Author {
    name: String,
}

mod parse_datetime {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer};

    const FORMAT: &'static str = "%FT%T%.3f%:z";
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Utc.datetime_from_str(&s, FORMAT)
            .map_err(serde::de::Error::custom)
    }
}

fn main() {
    let data: DiscordChatExporterJson = serde_json::from_reader(BufReader::new(
        File::open(env::var("JSON").expect("Failed to get JSON env"))
            .expect("Failed to open JSON file"),
    ))
    .expect("Failed to parse JSON");

    let config: Config = serde_yaml::from_reader(BufReader::new(
        File::open(env::var("CONFIG").expect("Failed to get CONFIG env"))
            .expect("Failed to open CONFIG file"),
    ))
    .expect("Failed to parse YAML");

    let mut prev_dt = chrono::MIN_DATETIME;
    let mut prev_user = "".to_string();

    let silence_threshold = Duration::minutes(config.silence_threshold_min);

    for message in data.messages {
        let body = (&config.transforms)
            .into_iter()
            .fold(message.content, |s, transform| match &transform {
                Transform::Regex(t) => t.from.replace_all(s.as_str(), t.to.as_str()).to_string(),
                Transform::ReplaceAll(t) => s.replace(t.from.as_str(), t.to.as_str()).to_string(),
            })
            .trim()
            .to_string();

        if body.len() == 0 {
            continue;
        }

        if silence_threshold < message.timestamp - prev_dt {
            print!("<silence>");
        }

        prev_dt = message.timestamp;

        if prev_user == message.author.name {
            print!("<sep>");
        } else {
            print!("\n");
        }

        prev_user = message.author.name;

        print!("{}", body);
    }
}
