use omoi::scheduler::{task_xml, TaskSpec};
#[test]
fn schedule_xml_contract() {
    let s = TaskSpec {
        sid: "S-1-5-21-123",
        start: "2026-11-03T08:00:00+08:00",
        helper: r"D:\Apps\omoiw.exe",
        home: r"D:\Data\omoi",
        directory: r"D:\Apps",
    };
    let xml = task_xml(&s).unwrap();
    for fragment in [
        "PT1H",
        "LogonTrigger",
        "InteractiveToken",
        "LeastPrivilege",
        "<Priority>7</Priority>",
        "<Hidden>true</Hidden>",
        "PT10M",
        "IgnoreNew",
        "S-1-5-21-123",
    ] {
        assert!(xml.contains(fragment), "{fragment}");
    }
}
#[test]
fn schedule_escapes_paths() {
    let s = TaskSpec {
        sid: "S-1",
        start: "now",
        helper: r"D:\A&B\omoiw.exe",
        home: r"D:\A&B\",
        directory: r"D:\A&B",
    };
    let xml = task_xml(&s).unwrap();
    assert!(xml.contains("A&amp;B"));
    assert!(xml.contains("--home &quot;D:\\A&amp;B\\\\&quot; collect"));
    let bad = TaskSpec {
        home: "bad\u{0}",
        ..s
    };
    assert!(task_xml(&bad).is_err());
}
