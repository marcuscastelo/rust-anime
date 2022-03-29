use crate::elements::*;
use crate::types::*;

trait AnimeDB {
    fn add_new_anime(&mut self, anime: &str) -> AnimeID;
    fn add_watch_entry(&mut self, entry: WatchEntry) -> Result<(), String>;
}

mod simple_database {  
    use super::*;

    struct _TitleInfo {
        name: String,
        watch_entries: Vec<WatchEntry>,
        // tags : ?
    }
    
    struct SimpleDatabase {
        titles: Vec<_TitleInfo>,
    }

    impl AnimeDB for SimpleDatabase {
        fn add_new_anime(&mut self, title: &str) -> AnimeID {
            let anime_id = self.titles.len();
            self.titles.push(_TitleInfo {
                name: title.to_string(),
                watch_entries: vec![],
            });
            anime_id
        }

        fn add_watch_entry(&mut self, entry: WatchEntry) -> Result<(), String> {
            let anime_id = entry.anime_id;
            if anime_id >= self.titles.len() {
                return Err(format!("Anime ID {} is out of range", anime_id));
            }
            //TODO: check if trying to add an episode thawt is less than the last one
            //TODO: accept tags for out-of-order entries
            self.titles[anime_id].watch_entries.push(entry);
            Ok(())
        }
    }
}