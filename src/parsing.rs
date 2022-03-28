use chrono::{NaiveDate, NaiveTime, Datelike};
use regex::{Regex};

#[path = "./elements.rs"]
mod elements;
use elements::*;

#[path = "./types.rs"]
mod types;
use types::*;

#[derive(Debug, PartialEq)]
struct ParsingContext {
    pub current_date: Option<NaiveDate>,
    pub current_anime: Option<AnimeID>,
}

impl ParsingContext {
    pub fn new() -> Self {
        Self {
            current_date: None,
            current_anime: None,
        }
    }
}

struct Database {
    animes: Vec<String>,
    watch_entries: Vec<WatchEntry>,
}

trait LineParser<T> {
    fn parse(&self, line: &str) -> Result<T, ParseDiagnostic>;
}

#[derive(Debug, PartialEq)]
struct DateLineParser;
impl LineParser<NaiveDate> for DateLineParser {
    fn parse(&self, line: &str) -> Result<NaiveDate, ParseDiagnostic> {
        let re = Regex::new(r"^\s*(\d{2}/\d{2}/\d{4})\s*(?://.*)?\s*$").unwrap();
        let caps = re.captures(line).ok_or_else(|| format!("Date parse error: \"{}\"", line))?;
        let date_str = match caps.get(1) {
            Some(s) => s.as_str(),
            None => return Err(format!("Date not found on line: {}", line))
        };

        match NaiveDate::parse_from_str(date_str, "%d/%m/%Y") {
            Ok(date) => Ok(date),
            Err(e) => Err(format!("Date parse error: {}", e))
        }
    }
}

#[derive(Debug, PartialEq)]
struct WatchLineParser<'a> {
    context: &'a ParsingContext,
}

impl LineParser<WatchEntry> for WatchLineParser<'_> {
    fn parse(&self, line: &str) -> Result<WatchEntry, ParseDiagnostic> {
        let current_date = self.context.current_date.ok_or_else(|| "No current date!".to_string())?;
        let current_anime = self.context.current_anime.ok_or_else(|| "No current anime!".to_string())?;

        let re = Regex::new(r"^([0-9]{2}:[0-9]{2})\s*-\s*([0-9]{2}:[0-9]{2})?\s+([0-9][0-9.]{1,}|--)?\s*(\{.*\})?\s*$").unwrap();
        let groups = re.captures(line).ok_or_else(|| format!("Line doesn't match regex: {}", line))?;

        let start_time = groups.get(1).ok_or_else(|| "No start time!".to_string())?.as_str();
        let end_time = groups.get(2).ok_or_else(|| "No end time!".to_string())?.as_str();
        let episode = groups.get(3).ok_or_else(|| "No episode number!".to_string())?.as_str();
        let company_match = groups.get(4);

        //Convert times to NaiveTime
        let start_time = NaiveTime::parse_from_str(start_time, "%H:%M").map_err(|e| format!("Invalid start time: {}", e))?;
        let end_time = NaiveTime::parse_from_str(end_time, "%H:%M").map_err(|e| format!("Invalid end time: {}", e))?; 

        //Account for current date in start and end times
        let start_time = NaiveDate::from_ymd(current_date.year(), current_date.month(), current_date.day()).and_time(start_time);
        let end_time = NaiveDate::from_ymd(current_date.year(), current_date.month(), current_date.day()).and_time(end_time);

        let episode = Episode::from_str(episode).map_err(|e| format!("Invalid episode: {}", e))?;

        let company = match company_match {
            Some(company) => Some(Company::from_str(company.as_str())?),
            None => None,
        };

        Ok(WatchEntry::new(
            current_anime,
            start_time,
            end_time,
            episode,
            company
        ))
    }
}

#[derive(Debug, PartialEq)]
struct TitleLineParser;

impl LineParser<String> for TitleLineParser {
    fn parse(&self, line: &str) -> Result<String, ParseDiagnostic> {
        let re = Regex::new(r"^\s*([a-zA-Z0-9][^\[\]\{\}]*):\s*(?://.*)?$").unwrap();
        if !re.is_match(line) {
            return Err(format!("Line doesn't match regex: \"{}\" instead of r\"^\\s*([a-zA-Z0-9][^{{[}}\\]]*):\\s*$\"", line));
        }

        let caps = re.captures(line).ok_or_else(|| format!("No matches found (missing semicolon?): \"{}\"", line))?;
        let anime_title = caps.get(1).ok_or_else(|| format!("Can't match anime title (missing semicolon?): \"{}\"", line))?.as_str();
        
        Ok(anime_title.to_string())
    }
}

#[cfg(test)]
mod tests {
    use chrono::prelude::*;

    use super::*;

    #[test]
    fn date_line_ok() {
        let line = "10/02/2022";
        let expected = NaiveDate::parse_from_str("10/02/2022", "%d/%m/%Y").unwrap();
        let date = DateLineParser.parse(line).unwrap();
        assert_eq!(date, expected);

        let line = "10/02/2022 // Some comment";
        let expected = NaiveDate::parse_from_str("10/02/2022", "%d/%m/%Y").unwrap();
        let date = DateLineParser.parse(line).unwrap();
        assert_eq!(date, expected);
    }

    #[test]
    fn date_line_fail() {
        let line = "Weird stuff";
        let dlp_res = DateLineParser.parse(line);
        assert!(dlp_res.is_err());

        let line = "10-02-2022";
        let dlp_res = DateLineParser.parse(line);
        assert!(dlp_res.is_err());

        let line = "10/02";
        let dlp_res = DateLineParser.parse(line);
        assert!(dlp_res.is_err());
    }

    #[test]
    fn watch_line_ok() {
        let line1 = "10:00 - 12:00 12 {Gary, Amim}";
        let line2 = "10:00 - 12:00 12 {Gary}";
        let line3 = "10:00 - 12:00 12";

        let context = ParsingContext{
            current_date: Some(NaiveDate::parse_from_str("10/02/2022", "%d/%m/%Y").unwrap()),
            current_anime: Some(1),
        };

        let watch_line = WatchLineParser{context: &context}.parse(line1).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("10:00", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("12:00", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from_str("12").unwrap());
        assert_eq!(watch_line.company, Some(Company::from_str("{Gary, Amim}").unwrap()));

        let watch_line = WatchLineParser{context: &context}.parse(line2).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("10:00", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("12:00", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from_str("12").unwrap());
        assert_eq!(watch_line.company, Some(Company::from_str("{Gary}").unwrap()));

        let watch_line = WatchLineParser{context: &context}.parse(line3).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("10:00", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("12:00", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from_str("12").unwrap());
        assert_eq!(watch_line.company, None);
    }

    #[test]
    fn anime_title_line_ok() {
        let line = "Erased:";
        let title_line = TitleLineParser.parse(line).unwrap();
        assert_eq!(title_line+":", line);

        let line = "Erased: The Animation:";
        let title_line = TitleLineParser.parse(line).unwrap();
        assert_eq!(title_line+":", line);

        let line = "86:";
        let title_line = TitleLineParser.parse(line).unwrap();
        assert_eq!(title_line+":", line);

        let line = "86: The Animation:";
        let title_line = TitleLineParser.parse(line).unwrap();
        assert_eq!(title_line+":", line);

        let line = "Erased: The Animation: (TV):";
        let title_line = TitleLineParser.parse(line).unwrap();
        assert_eq!(title_line+":", line);

        let line = "Erased: The Animation 2: (TV):";
        let title_line = TitleLineParser.parse(line).unwrap();
        assert_eq!(title_line+":", line);

        let line = "Re:zero kara Hajimeru isekai Seikatsu:";
        let title_line = TitleLineParser.parse(line).unwrap();
        assert_eq!(title_line+":", line);

        let line = "Re:zero kara Hajimeru isekai Seikatsu 2:";
        let title_line = TitleLineParser.parse(line).unwrap();
        assert_eq!(title_line+":", line);
    }

    #[test]
    fn real_sample() {
        let line = "19/03/2022";
        let dateline = DateLineParser.parse(line).unwrap();
        assert_eq!(dateline, NaiveDate::parse_from_str("19/03/2022", "%d/%m/%Y").unwrap());

        let line = "Evangelion: 1.0 You Are (Not) Alone: // 1.11";
        let anime_title_line = TitleLineParser.parse(line).unwrap();
        assert_eq!(anime_title_line, "Evangelion: 1.0 You Are (Not) Alone");        

        let context = ParsingContext {
            current_date: Some(NaiveDate::parse_from_str("10/02/2022", "%d/%m/%Y").unwrap()),
            current_anime: Some(1),
        };

        let line = "16:40 - 18:24 01 {Vinicius Russo}";
        let watch_line = WatchLineParser{context: &context}.parse(line).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("16:40", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("18:24", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from_str("01").unwrap());
        assert_eq!(watch_line.company, Some(Company::from_str("{Vinicius Russo}").unwrap()));

        let line = "One Pace: Reverie:";
        let anime_title_line = TitleLineParser.parse(line).unwrap();
        assert_eq!(anime_title_line, "One Pace: Reverie");

        let line = "20:09 - 20:46 01 {Lucas Romero}";
        let watch_line = WatchLineParser{context: &context}.parse(line).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("20:09", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("20:46", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from_str("01").unwrap());
        assert_eq!(watch_line.company, Some(Company::from_str("{Lucas Romero}").unwrap()));

        let line = "20:46 - 21:26 02 {Lucas Romero}";
        let watch_line = WatchLineParser{context: &context}.parse(line).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("20:46", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("21:26", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from_str("02").unwrap());
        assert_eq!(watch_line.company, Some(Company::from_str("{Lucas Romero}").unwrap()));

        let line = "21:27 - 22:04 03 {Lucas Romero}";
        let watch_line = WatchLineParser{context: &context}.parse(line).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("21:27", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("22:04", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from_str("03").unwrap());
        assert_eq!(watch_line.company, Some(Company::from_str("{Lucas Romero}").unwrap()));

        let line = "One Pace: Wano:";
        let anime_title_line = TitleLineParser.parse(line).unwrap();
        assert_eq!(anime_title_line, "One Pace: Wano");

        let line = "22:11 - 22:35 01 {Lucas Romero}";
        let watch_line = WatchLineParser{context: &context}.parse(line).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("22:11", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("22:35", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from_str("01").unwrap());
        assert_eq!(watch_line.company, Some(Company::from_str("{Lucas Romero}").unwrap()));
        
        let line = "22:44 - 23:17 02";
        let watch_line = WatchLineParser{context: &context}.parse(line).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("22:44", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("23:17", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from_str("02").unwrap());
        assert_eq!(watch_line.company, None);
    }
}