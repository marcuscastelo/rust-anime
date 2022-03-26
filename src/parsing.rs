use chrono::{NaiveDate, NaiveTime};
use regex::{Regex};
type ParseDiagnostic = String;

#[path ="./elements.rs"]
mod elements;
use elements::*;

struct Database {
    animes: Vec<String>,
    watch_entries: Vec<WatchEntry>,
}

trait LineParser<T> {
    fn parse(line: &str) -> Result<T, ParseDiagnostic>;
}

#[derive(Debug, PartialEq)]
struct DateLineParser;
impl LineParser<NaiveDate> for DateLineParser {
    fn parse(line: &str) -> Result<NaiveDate, ParseDiagnostic> {
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
struct WatchLineParser {
    start_time: NaiveTime,
    end_time: NaiveTime,

    episode: Episode,

    company: Option<Company>,
}

impl WatchLineParser {
    fn from_str(line: &str) -> Result<Self, ParseDiagnostic> {
        let re = Regex::new(r"^([0-9]{2}:[0-9]{2})\s*-\s*([0-9]{2}:[0-9]{2})?\s+([0-9][0-9.]{1,}|--)?\s*(\{.*\})?\s*$").unwrap();
        let groups = re.captures(line).expect("Line doesn't match regex");

        let start_time = groups.get(1).expect("No start time!").as_str();
        let end_time = groups.get(2).expect("No end time!").as_str();
        let episode = groups.get(3).expect("No episode number!").as_str();
        let company_match = groups.get(4);

        let start_time = NaiveTime::parse_from_str(start_time, "%H:%M").expect("Invalid start time");
        let end_time = NaiveTime::parse_from_str(end_time, "%H:%M").expect("Invalid end time");

        let episode = Episode::from_str(episode)?;

        let company = match company_match {
            Some(company) => Some(Company::from_str(company.as_str())?),
            None => None,
        };

        Ok(Self { start_time, end_time, episode, company })
    }
}

impl LineParser<WatchEntry> for WatchLineParser {
    fn parse(line: &str) -> Result<WatchEntry, ParseDiagnostic> {
        let watch_line = WatchLineParser::from_str(line)?;
        Ok(WatchEntry::new(
            wa
            watch_line.episode,
            watch_line.company
        ))
    }
}

#[derive(Debug, PartialEq)]
struct AnimeTitleLine {
    anime_title: String
}

impl AnimeTitleLine {
    fn from_str(line: &str) -> Result<Self, ParseDiagnostic> {
        let re = Regex::new(r"^\s*([a-zA-Z0-9][^\[\]\{\}]*):\s*(?://.*)?$").unwrap();
        if !re.is_match(line) {
            return Err(format!("Line doesn't match regex: \"{}\" instead of r\"^\\s*([a-zA-Z0-9][^{{[}}\\]]*):\\s*$\"", line));
        }

        let caps = re.captures(line).ok_or_else(|| format!("No matches found (missing semicolon?): \"{}\"", line))?;
        let anime_title = caps.get(1).ok_or_else(|| format!("Can't match anime title (missing semicolon?): \"{}\"", line))?.as_str();
        
        Ok(Self { anime_title: anime_title.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{prelude::*};

    use super::*;

    #[test]
    fn date_line_ok() {
        let line = "10/02/2022";
        let expected = NaiveDate::parse_from_str("10/02/2022", "%d/%m/%Y").unwrap();
        let date = DateLineParser::parse(line).unwrap();
        assert_eq!(date, expected);

        let line = "10/02/2022 // Some comment";
        let expected = NaiveDate::parse_from_str("10/02/2022", "%d/%m/%Y").unwrap();
        let date = DateLineParser::parse(line).unwrap();
        assert_eq!(date, expected);
    }

    #[test]
    fn date_line_fail() {
        let line = "Weird stuff";
        let dlp_res = DateLineParser::parse(line);
        assert!(dlp_res.is_err());

        let line = "10-02-2022";
        let dlp_res = DateLineParser::parse(line);
        assert!(dlp_res.is_err());

        let line = "10/02";
        let dlp_res = DateLineParser::parse(line);
        assert!(dlp_res.is_err());
    }

    #[test]
    fn watch_line_ok() {
        let line1 = "10:00 - 12:00 12 {Gary, Amim}";
        let line2 = "10:00 - 12:00 12 {Gary}";
        let line3 = "10:00 - 12:00 12";

        let watch_line = WatchLineParser::from_str(line1).unwrap();
        assert_eq!(watch_line.start_time, NaiveTime::parse_from_str("10:00", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time, NaiveTime::parse_from_str("12:00", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from_str("12").unwrap());
        assert_eq!(watch_line.company, Some(Company::from_str("{Gary, Amim}").unwrap()));

        let watch_line = WatchLineParser::from_str(line2).unwrap();
        assert_eq!(watch_line.start_time, NaiveTime::parse_from_str("10:00", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time, NaiveTime::parse_from_str("12:00", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from_str("12").unwrap());
        assert_eq!(watch_line.company, Some(Company::from_str("{Gary}").unwrap()));

        let watch_line = WatchLineParser::from_str(line3).unwrap();
        assert_eq!(watch_line.start_time, NaiveTime::parse_from_str("10:00", "%H:%M").unwrap());
        assert_eq!(watch_line.end_time, NaiveTime::parse_from_str("12:00", "%H:%M").unwrap());
        assert_eq!(watch_line.episode, Episode::from_str("12").unwrap());
        assert_eq!(watch_line.company, None);
    }

    #[test]
    fn anime_title_line_ok() {
        let title1 = "Erased:";
        let title_line = AnimeTitleLine::from_str(title1).unwrap();
        assert_eq!(title_line.anime_title+":", title1);

        let title2 = "Erased: The Animation:";
        let title_line = AnimeTitleLine::from_str(title2).unwrap();
        assert_eq!(title_line.anime_title+":", title2);

        let title3 = "86:";
        let title_line = AnimeTitleLine::from_str(title3).unwrap();
        assert_eq!(title_line.anime_title+":", title3);

        let title4 = "86: The Animation:";
        let title_line = AnimeTitleLine::from_str(title4).unwrap();
        assert_eq!(title_line.anime_title+":", title4);

        let title5 = "Erased: The Animation: (TV):";
        let title_line = AnimeTitleLine::from_str(title5).unwrap();
        assert_eq!(title_line.anime_title+":", title5);

        let title6 = "Erased: The Animation 2: (TV):";
        let title_line = AnimeTitleLine::from_str(title6).unwrap();
        assert_eq!(title_line.anime_title+":", title6);

        let title7 = "Re:zero kara Hajimeru isekai Seikatsu:";
        let title_line = AnimeTitleLine::from_str(title7).unwrap();
        assert_eq!(title_line.anime_title+":", title7);

        let title8 = "Re:zero kara Hajimeru isekai Seikatsu 2:";
        let title_line = AnimeTitleLine::from_str(title8).unwrap();
        assert_eq!(title_line.anime_title+":", title8);
    }

    #[test]
    fn real_sample() {
        let line = "19/03/2022";
        let dateline = DateLineParser::parse(line).unwrap();
        println!("{:?}", dateline);


        let line = "";
        let line = "Evangelion: 1.0 You Are (Not) Alone: // 1.11";
        let anime_title_line = AnimeTitleLine::from_str(line).unwrap();
        println!("{:?}", anime_title_line);

        let line = "";
        let line = "16:40 - 18:24 01 {Vinicius Russo}";
        let watch_line = WatchLineParser::from_str(line).unwrap();
        println!("{:?}", watch_line);

        let line = "";

        let line = "One Pace: Reverie:";
        let anime_title_line = AnimeTitleLine::from_str(line).unwrap();
        println!("{:?}", anime_title_line);


        let line = "";
        let line = "20:09 - 20:46 01 {Lucas Romero}";
        let watch_line = WatchLineParser::from_str(line).unwrap();
        println!("{:?}", watch_line);

        let line = "20:46 - 21:26 02 {Lucas Romero}";
        let watch_line = WatchLineParser::from_str(line).unwrap();
        println!("{:?}", watch_line);

        let line = "21:27 - 22:04 03 {Lucas Romero}";
        let line = "";
        let line = "One Pace: Wano:";
        let line = "";
        let line = "22:11 - 22:35 01 {Lucas Romero}";
        let line = "22:44 - 23:17 02 {Lucas Romero}";
    }
}