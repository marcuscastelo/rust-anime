use chrono::{NaiveDateTime};
use regex::{Regex};

#[path ="./types.rs"]
mod types;
use types::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Episode {
    number: i32, //TODO: support different episode types (e.g. "1.5", "[1 -> 5]", "1 -> 5", "[1,2,3,4,5]", etc.)
}

impl Episode {
    pub fn from(ep_str: &str) -> Result<Self, Diagnostic> {
        let number = ep_str.parse().map_err(|_| format!("Invalid episode number: {}", ep_str))?;
        Ok(Self { number })
    } 
}


#[derive(Debug, PartialEq, Clone)]
pub struct Company {
    names: Vec<String>
}


impl Company {
    pub fn from_str(company_str: &str) -> Result<Self, Diagnostic> {
        if !Regex::new(r"^\{(.*)\}$").unwrap().is_match(company_str) {
            return Err(format!("String does not match company format: \"{}\" instead of r\"{{(.*)}}\"", company_str));
        }

        // Drop the braces
        let company_str = &company_str[1..company_str.len()-1];
        let company_str = company_str.trim();

        let names = match company_str.trim() {
            "" => vec![],
            _ => company_str.split(',')
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_string)
                    .collect()
        };
        Ok(Self { names })
    }

    fn iter(&self) -> impl Iterator<Item = &String> {
        self.names.iter()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct WatchEntry {
    pub anime_id: AnimeID,   
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub episode: Episode,
    pub company: Option<Company>,
}

impl WatchEntry {
    pub fn new(anime_id: AnimeID, start_time: NaiveDateTime, end_time: NaiveDateTime, episode: Episode, company: Option<Company>) -> Self {
        Self {
            anime_id,
            start_time,
            end_time,
            episode,
            company,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn company_from_str() {
        let company = Company::from_str("{Konami,Square Enix}").unwrap();
        let expected = Company { names: vec!["Konami".to_string(), "Square Enix".to_string()] };
        assert_eq!(company, expected);

        let company = Company::from_str("{Konami}").unwrap();
        let expected = Company { names: vec!["Konami".to_string()] };
        assert_eq!(company, expected);

        let company = Company::from_str("{}").unwrap();
        let expected = Company { names: vec![] };
        assert_eq!(company, expected);

        let company = Company::from_str("{Konami,Square Enix,}").unwrap();
        let expected = Company { names: vec!["Konami".to_string(), "Square Enix".to_string()] };
        assert_eq!(company, expected);

        // let company = Company::from_str("");
        // let expected = Company { names: vec![] };
        // assert_eq!(company, expected);
    }

    #[test]
    fn episode_from_str() {
        let episode = Episode::from("1").unwrap();
        let expected = Episode { number: 1 };
        assert_eq!(episode, expected);

        let episode = Episode::from("01").unwrap();
        let expected = Episode { number: 1 };
        assert_eq!(episode, expected);

        let episode = Episode::from("001").unwrap();
        let expected = Episode { number: 1 };
        assert_eq!(episode, expected);

        let episode = Episode::from("-1").unwrap();
        let expected = Episode { number: -1 };
        assert_eq!(episode, expected);

        let episode = Episode::from("-01").unwrap();
        let expected = Episode { number: -1 };
        assert_eq!(episode, expected);

        let episode = Episode::from("a");
        assert!(episode.is_err());

        let episode = Episode::from("1a");
        assert!(episode.is_err());

        let episode = Episode::from("1.1");
        assert!(episode.is_err());
    }
}

