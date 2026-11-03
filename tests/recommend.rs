use rusqlite::Connection;
#[test]
fn recommendations_are_local_and_explained() {
    let mut c = Connection::open_in_memory().unwrap();
    omoi::db::migrate(&mut c).unwrap();
    c.execute_batch("INSERT INTO items(id,platform,external_id,title,duration_s) VALUES(1,'youtube','abcdefghijk','Learn Rust',100),(2,'youtube','bcdefghijkl','Rust ownership',200),(3,'youtube','cdefghijklm','Other',NULL);INSERT INTO item_tags VALUES(1,'coding','user'),(2,'coding','user');INSERT INTO views(item_id,started_at,source) VALUES(2,0,'chrome');INSERT INTO sessions(item_id,start,end,played_s,max_position_s,source) VALUES(1,0,40,40,40,'userscript');").unwrap();
    let r = omoi::recommend::shelves(&c, 4000000, Some(1), 300).unwrap();
    assert_eq!(r.continue_watching[0].id, 1);
    assert_eq!(r.similar[0].id, 2);
    assert!(r.similar[0].reason.contains("tag"));
    assert!(r.timebox.iter().all(|i| i.id != 3));
    assert!(!r.unavailable.is_empty());
}
