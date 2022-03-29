use crate::elements::*;
use crate::types::*;

trait AnimeDB {
    fn add_new_anime(&mut self, anime: &str) -> Result<AnimeID, String>;
    fn add_watch_entry(&mut self, entry: WatchEntry) -> Result<(), String>;

    fn find_anime_by_id<'db>(&'db mut self, anime_id: AnimeID) -> Option<&'db mut Anime>;
    fn find_anime_by_name<'db>(&'db mut self, name: &str) -> Option<&'db mut Anime>;
}

#[derive(Debug, PartialEq, Clone)]
struct Anime {
    id: AnimeID,
    name: String,
    watch_entries : Vec<WatchEntry>,
}

impl Anime {
    pub fn new(id: AnimeID, name: String) -> Self {
        Self {
            id,
            name,
            watch_entries: vec![],
        }
    }

    pub fn watch_entries(&self) -> impl Iterator<Item = &WatchEntry> {
        self.watch_entries.iter()
    }
}

mod simple_database {  
    use std::{collections::HashMap};

    use super::*;

    pub struct SimpleDatabase {
        anime_map: HashMap<AnimeID, Anime>
    }

    impl SimpleDatabase {
        pub fn new() -> Self {
            Self {
                anime_map: HashMap::new(),
            }
        }
    }

    impl AnimeDB for SimpleDatabase {
        fn add_new_anime(&mut self, title: &str) -> Result<AnimeID, String> {
            
            match self.find_anime_by_name(title) {
                Some(_) => Err(format!("Anime with name {} already exists", title)),
                None => {
                    let anime_id = self.anime_map.len();
                    let anime = Anime::new(anime_id, title.to_string());
                    self.anime_map.insert(anime_id, anime);
                    Ok(anime_id)
                }
            }

        }

        fn add_watch_entry(&mut self, entry: WatchEntry) -> Result<(), String> {
            let anime_id = entry.anime_id;
            if anime_id >= self.anime_map.len() {
                return Err(format!("Anime ID {} is out of range", anime_id));
            }
            
            let anime = 
                self.find_anime_by_id(anime_id)
                .ok_or_else(|| format!("Anime ID {} not found", anime_id))?;

            anime.watch_entries.push(entry);
            Ok(())
        }

        fn find_anime_by_id(&mut self, anime_id: AnimeID) -> Option<&mut Anime> {
            self.anime_map.get_mut(&anime_id)
        }

        fn find_anime_by_name(&mut self, name: &str) -> Option<&mut Anime> {
            self.anime_map.values_mut().find(|anime| anime.name == name)
        }


    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;

    use super::*;

    #[test]
    fn db_empty_anime_not_found() {
        let mut db = simple_database::SimpleDatabase::new();

        let anime_id = 1;
        assert_eq!(db.find_anime_by_id(anime_id), None);
    }

    #[test]
    fn added_anime_found() {
        let mut db = simple_database::SimpleDatabase::new();

        let anime_id = db.add_new_anime("My Anime").unwrap();
        assert_eq!(db.find_anime_by_id(anime_id), Some(&mut Anime::new(anime_id, "My Anime".to_string())));
    }

    #[test]
    fn add_existing_anime_fails() {
        let mut db = simple_database::SimpleDatabase::new();

        let anime_id = db.add_new_anime("My Anime");
        assert!(db.add_new_anime("My Anime").is_err(),"Adding existing anime should fail, but was successful");
    }

    #[test]
    fn add_two_animes_both_ok() {
        let mut db = simple_database::SimpleDatabase::new();

        let anime_id_1 = db.add_new_anime("My Anime 1").unwrap();
        let anime_id_2 = db.add_new_anime("My Anime 2").unwrap();

        assert_eq!(db.find_anime_by_id(anime_id_1), Some(&mut Anime::new(anime_id_1, "My Anime 1".to_string())));
        assert_eq!(db.find_anime_by_id(anime_id_2), Some(&mut Anime::new(anime_id_2, "My Anime 2".to_string())));
        assert_ne!(anime_id_1, anime_id_2);
    }

    #[test]
    fn add_two_animes_third_doesnt_exist() {
        let mut db = simple_database::SimpleDatabase::new();

        let anime_id_1 = db.add_new_anime("My Anime 1").unwrap();
        let anime_id_2 = db.add_new_anime("My Anime 2").unwrap();

        let anime_title = "My Anime 3";
        assert_eq!(db.find_anime_by_name(anime_title), None);
    }

    #[test]
    fn watch_entries_start_empty() {
        let mut db = simple_database::SimpleDatabase::new();

        let anime_id = db.add_new_anime("My Anime").unwrap();
        let anime = db.find_anime_by_id(anime_id).unwrap();
        assert_eq!(anime.watch_entries.len(), 0);
    }

    #[test]
    fn add_watch_entry_ok() {
        let mut db = simple_database::SimpleDatabase::new();

        let anime_id = db.add_new_anime("My Anime").unwrap();
        
        let entry = WatchEntry::new(
            anime_id,
            NaiveDateTime::from_timestamp(0, 0), 
            NaiveDateTime::from_timestamp(1, 0),
            Episode::from("1").unwrap(),
            None,
        );

        let anime = db.find_anime_by_id(anime_id).unwrap();
        assert_eq!(anime.watch_entries.len(), 0);
        
        db.add_watch_entry(entry.clone()).unwrap();
        
        let anime = db.find_anime_by_id(anime_id).unwrap();
        assert_eq!(anime.watch_entries.len(), 1);
        assert_eq!(anime.watch_entries[0], entry);
    }

    #[test]
    fn watch_entry_keeps_insertion_order() {
        let mut db = simple_database::SimpleDatabase::new();

        let anime_id = db.add_new_anime("My Anime").unwrap();
        
        let entry_1 = WatchEntry::new(
            anime_id,
            NaiveDateTime::from_timestamp(0, 0), 
            NaiveDateTime::from_timestamp(1, 0),
            Episode::from("1").unwrap(),
            None,
        );

        let entry_2 = WatchEntry::new(
            anime_id,
            NaiveDateTime::from_timestamp(0, 0), 
            NaiveDateTime::from_timestamp(1, 0),
            Episode::from("2").unwrap(),
            None,
        );

        let entry_3 = WatchEntry::new(
            anime_id,
            NaiveDateTime::from_timestamp(0, 0), 
            NaiveDateTime::from_timestamp(1, 0),
            Episode::from("3").unwrap(),
            None,
        );

        db.add_watch_entry(entry_1.clone()).unwrap();
        db.add_watch_entry(entry_2.clone()).unwrap();
        db.add_watch_entry(entry_3.clone()).unwrap();

        let anime = db.find_anime_by_id(anime_id).unwrap();
        assert_eq!(anime.watch_entries.len(), 3);
        assert_eq!(anime.watch_entries[0], entry_1);
        assert_eq!(anime.watch_entries[1], entry_2);
        assert_eq!(anime.watch_entries[2], entry_3);
    }

    #[test]
    fn insert_watch_entry_doesnt_affect_other_animes() {
        let mut db = simple_database::SimpleDatabase::new();

        let anime_id_1 = db.add_new_anime("My Anime 1").unwrap();
        let anime_id_2 = db.add_new_anime("My Anime 2").unwrap();

        let entry_1 = WatchEntry::new(
            anime_id_1,
            NaiveDateTime::from_timestamp(0, 0), 
            NaiveDateTime::from_timestamp(1, 0),
            Episode::from("1").unwrap(),
            None,
        );

        let entry_2 = WatchEntry::new(
            anime_id_2,
            NaiveDateTime::from_timestamp(0, 0), 
            NaiveDateTime::from_timestamp(1, 0),
            Episode::from("2").unwrap(),
            None,
        );

        db.add_watch_entry(entry_1.clone()).unwrap();
        db.add_watch_entry(entry_2.clone()).unwrap();

        let anime_1 = db.find_anime_by_id(anime_id_1).unwrap();
        assert_eq!(anime_1.watch_entries.len(), 1);
        assert_eq!(anime_1.watch_entries[0], entry_1);

        let anime_2 = db.find_anime_by_id(anime_id_2).unwrap();
        assert_eq!(anime_2.watch_entries.len(), 1);
        assert_eq!(anime_2.watch_entries[0], entry_2);
    }
}