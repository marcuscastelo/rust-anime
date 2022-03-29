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
    current_date: Option<NaiveDate>,
    current_anime: Option<AnimeID>,
    last_watch_entry: Option<WatchEntry>,
    last_company: Option<Company>,
    // current_anime_tag
    // current_session_tag
    // current_episode_tag
}

impl ParsingContext {
    pub fn new() -> Self {
        Self {
            current_date: None,
            current_anime: None,
            last_watch_entry: None,
            last_company: None,
        }
    }

    pub fn notify_new_current_date(&mut self, date: NaiveDate) -> Result<(), String> {
        if let Some(current_date) = self.current_date {
            if current_date >= date {
                return Err(format!("Current date {} is earlier or equal than previous date {}", date, current_date));
            }
        }

        self.current_date = Some(date);
        self.current_anime = None;
        self.last_watch_entry = None;

        Ok(())
    }

    pub fn notify_new_current_anime(&mut self, anime_id: AnimeID) -> Result<(), String> {
        self.current_anime = Some(anime_id);
        self.last_watch_entry = None;
        Ok(())
    }

    pub fn notify_new_watch_entry(&mut self, entry: WatchEntry) -> Result<(), String> {
        self.last_watch_entry = match self.last_watch_entry {
            Some(ref last_entry) => {
                assert_eq!(last_entry.anime_id, entry.anime_id, "Anime ID mismatch");
                Some(entry)
            },
            None => {
                Some(entry)
            },
        };

        Ok(())
    }

    pub fn notify_new_company(&mut self, company: Option<Company>) -> Result<(), String> {
        self.last_company = company;

        Ok(())
    }
}

struct Database {
    animes: Vec<String>,
    watch_entries: Vec<WatchEntry>,
}

trait LineParser<T> {
    fn parse(&mut self, line: &str) -> Result<T, ParseDiagnostic>;
}

#[derive(Debug, PartialEq)]
struct DateLineParser;
impl LineParser<NaiveDate> for DateLineParser {
    fn parse(&mut self, line: &str) -> Result<NaiveDate, ParseDiagnostic> {
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
    context: &'a mut ParsingContext,
}

impl LineParser<WatchEntry> for WatchLineParser<'_> {
    fn parse(&mut self, line: &str) -> Result<WatchEntry, ParseDiagnostic> {
        let mut current_date = self.context.current_date.ok_or_else(|| "No current date!".to_string())?;
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
        
        //TODO: check if trying to add an episode that is less than the last one
        //TODO: accept tags for out-of-order entries

        let anime_id = self.context.current_anime.ok_or("No current anime in context!".to_string())?;

        //Special case for midnight
        let (mut start_date, mut end_date) = (current_date, current_date);
        {
            // Start after midnight with previous watch entry on yesterday
            if let Some(ref last_entry) = self.context.last_watch_entry {
                if last_entry.end_time.time() > start_time {
                    start_date = current_date.succ();
                    end_date = start_date;
                    current_date = current_date.succ();
                    assert_eq!(last_entry.anime_id, anime_id, "Anime ID mismatch");

                    self.context.notify_new_current_date(current_date)?;
                    self.context.notify_new_current_anime(anime_id)?;
                    //TODO: instead of re-adding all context after setting date, 
                    //TODO: set date without resetting old context
                }
            }
            
            //Start before midnight and end after midnight
            if end_time < start_time {
                start_date = current_date;
                end_date = current_date.succ();
                current_date = current_date.succ();

                self.context.notify_new_current_date(current_date)?;
                self.context.notify_new_current_anime(anime_id)?;
                //TODO: instead of re-adding all context after setting date, 
                //TODO: set date without resetting old context
            }

        
        }
        drop(current_date);

        //Account for current date in start and end times
        let start_time = NaiveDate::from_ymd(start_date.year(), start_date.month(), start_date.day()).and_time(start_time);
        let end_time = NaiveDate::from_ymd(end_date.year(), end_date.month(), end_date.day()).and_time(end_time);

        let episode = Episode::from(episode).map_err(|e| format!("Invalid episode: {}", e))?;

        let company = match company_match {
            Some(company) => Some(Company::from_str(company.as_str())?),
            None => None,
        };

        let watch_entry = WatchEntry::new(
            current_anime,
            start_time,
            end_time,
            episode,
            company
        );

        self.context.notify_new_watch_entry(watch_entry.clone())?;
        
        Ok(watch_entry)
    }
}

#[derive(Debug, PartialEq)]
struct TitleLineParser;

impl LineParser<String> for TitleLineParser {
    fn parse(&mut self, line: &str) -> Result<String, ParseDiagnostic> {
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

        let mut context = ParsingContext{
            current_date: Some(NaiveDate::parse_from_str("10/02/2022", "%d/%m/%Y").unwrap()),
            current_anime: Some(1),
            last_company: None,
            last_watch_entry: None,
        };

        let watch_line = WatchLineParser{context: &mut context}.parse(line1).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("10:00", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("12:00", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from("12").unwrap());
        assert_eq!(watch_line.company, Some(Company::from_str("{Gary, Amim}").unwrap()));

        let watch_line = WatchLineParser{context: &mut context}.parse(line2).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("10:00", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("12:00", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from("12").unwrap());
        assert_eq!(watch_line.company, Some(Company::from_str("{Gary}").unwrap()));

        let watch_line = WatchLineParser{context: &mut context}.parse(line3).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("10:00", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("12:00", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from("12").unwrap());
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
    fn midnight_last() {
        let mut context = ParsingContext {
            current_anime: Some(1),
            current_date: Some(NaiveDate::parse_from_str("10/02/2022", "%d/%m/%Y").unwrap()),
            last_company: None,
            last_watch_entry: None,
        };

        let line1 = "23:00 - 23:40 12";
        let watch_line1 = WatchLineParser{context: &mut context}.parse(line1).unwrap();
        let line2 = "00:00 - 00:10 13";
        let watch_line2 = WatchLineParser{context: &mut context}.parse(line2).unwrap();

        assert_eq!(watch_line1.start_time.date(), watch_line1.end_time.date(), "Dates should be the same");
        assert_eq!(watch_line2.start_time.date(), watch_line2.end_time.date(), "Dates should be the same");
        assert!(watch_line1.start_time.date() != watch_line2.start_time.date(), "Dates should be different");
    }

    #[test]
    fn midnight_traverse() {
        let initial_date = NaiveDate::parse_from_str("10/02/2022", "%d/%m/%Y").unwrap();
        let mut context = ParsingContext {
            current_anime: Some(1),
            current_date: Some(NaiveDate::parse_from_str("10/02/2022", "%d/%m/%Y").unwrap()),
            last_company: None,
            last_watch_entry: None,
        };

        let line1 = "23:40 - 00:20 12";
        let watch_line1 = WatchLineParser{context: &mut context}.parse(line1).unwrap();

        assert_eq!(watch_line1.start_time.date(), initial_date, "Dates should be the same");
        assert_eq!(watch_line1.end_time.date(), initial_date.succ(), "Date should be the next day");
        assert_eq!(context.current_date, Some(initial_date.succ()), "Date should be incremented");

        let line2 = "00:20 - 00:30 13";
        let watch_line2 = WatchLineParser{context: &mut context}.parse(line2).unwrap();

        assert_eq!(watch_line2.start_time.date(), initial_date.succ(), "Dates should be the next day");
        assert_eq!(watch_line2.end_time.date(), initial_date.succ(), "Date should be the next day");
        assert_eq!(context.current_date, Some(initial_date.succ()), "Date should be incremented");

    }

    #[test]
    fn midnight_last_and_traverse() {
        let initial_date = NaiveDate::parse_from_str("10/02/2022", "%d/%m/%Y").unwrap();
        let mut context = ParsingContext {
            current_anime: Some(1),
            current_date: Some(NaiveDate::parse_from_str("10/02/2022", "%d/%m/%Y").unwrap()),
            last_company: None,
            last_watch_entry: None,
        };

        let line0 = "23:00 - 23:40 12";
        let watch_line0 = WatchLineParser{context: &mut context}.parse(line0).unwrap();

        assert_eq!(watch_line0.start_time.date(), initial_date, "Dates should be the same");
        assert_eq!(watch_line0.end_time.date(), initial_date, "Dates should be the same");
        assert_eq!(context.current_date, Some(initial_date), "Date should be the same");

        let line1 = "23:00 - 02:10 12"; // Traverse to next day
        let watch_line1 = WatchLineParser{context: &mut context}.parse(line1).unwrap();

        assert_eq!(watch_line1.start_time.date(), initial_date, "Dates should be the same");
        assert_eq!(watch_line1.end_time.date(), initial_date.succ(), "Date should be the next day");
        assert_eq!(context.current_date, Some(initial_date.succ()), "Date should be incremented");

        let line2 = "02:10 - 00:00 13"; // Traverse to next day
        let watch_line2 = WatchLineParser{context: &mut context}.parse(line2).unwrap();

        assert_eq!(watch_line2.start_time.date(), initial_date.succ(), "Dates should be the next day");
        assert_eq!(watch_line2.end_time.date(), initial_date.succ().succ(), "Date should be the nexts next day");
        assert_eq!(context.current_date, Some(initial_date.succ().succ()), "Date should be incremented twice");

    }

    #[test]
    fn real_sample() {
        let line = "19/03/2022";
        let dateline = DateLineParser.parse(line).unwrap();
        assert_eq!(dateline, NaiveDate::parse_from_str("19/03/2022", "%d/%m/%Y").unwrap());

        let line = "Evangelion: 1.0 You Are (Not) Alone: // 1.11";
        let anime_title_line = TitleLineParser.parse(line).unwrap();
        assert_eq!(anime_title_line, "Evangelion: 1.0 You Are (Not) Alone");        

        let mut context = ParsingContext {
            current_date: Some(NaiveDate::parse_from_str("10/02/2022", "%d/%m/%Y").unwrap()),
            current_anime: Some(1),
            last_company: None,
            last_watch_entry: None,
        };

        let line = "16:40 - 18:24 01 {Vinicius Russo}";
        let watch_line = WatchLineParser{context: &mut context}.parse(line).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("16:40", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("18:24", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from("01").unwrap());
        assert_eq!(watch_line.company, Some(Company::from_str("{Vinicius Russo}").unwrap()));

        let line = "One Pace: Reverie:";
        let anime_title_line = TitleLineParser.parse(line).unwrap();
        assert_eq!(anime_title_line, "One Pace: Reverie");

        let line = "20:09 - 20:46 01 {Lucas Romero}";
        let watch_line = WatchLineParser{context: &mut context}.parse(line).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("20:09", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("20:46", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from("01").unwrap());
        assert_eq!(watch_line.company, Some(Company::from_str("{Lucas Romero}").unwrap()));

        let line = "20:46 - 21:26 02 {Lucas Romero}";
        let watch_line = WatchLineParser{context: &mut context}.parse(line).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("20:46", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("21:26", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from("02").unwrap());
        assert_eq!(watch_line.company, Some(Company::from_str("{Lucas Romero}").unwrap()));

        let line = "21:27 - 22:04 03 {Lucas Romero}";
        let watch_line = WatchLineParser{context: &mut context}.parse(line).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("21:27", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("22:04", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from("03").unwrap());
        assert_eq!(watch_line.company, Some(Company::from_str("{Lucas Romero}").unwrap()));

        let line = "One Pace: Wano:";
        let anime_title_line = TitleLineParser.parse(line).unwrap();
        assert_eq!(anime_title_line, "One Pace: Wano");

        let line = "22:11 - 22:35 01 {Lucas Romero}";
        let watch_line = WatchLineParser{context: &mut context}.parse(line).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("22:11", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("22:35", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from("01").unwrap());
        assert_eq!(watch_line.company, Some(Company::from_str("{Lucas Romero}").unwrap()));
        
        let line = "22:44 - 23:17 02";
        let watch_line = WatchLineParser{context: &mut context}.parse(line).unwrap();
        assert_eq!(watch_line.start_time.time(), NaiveTime::parse_from_str("22:44", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time.time(), NaiveTime::parse_from_str("23:17", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from("02").unwrap());
        assert_eq!(watch_line.company, None);
    }
}