use demo_picker::{ImmichClient, Picker};

#[test]
fn verify_random_picker() {
    let client = ImmichClient::from_env().expect("read stub address from environment");
    let mut picker = Picker::new();
    match client.search_random() {
        Ok(response) => picker.add_response(&response),
        Err(_) => {}
    }
    let got: Vec<String> = picker.selection().to_vec();
    assert_eq!(
        got,
        vec!["asset-r1".to_string(), "asset-r2".to_string()],
        "random picker ids"
    );
}

#[test]
fn verify_smart_guard() {
    let client = ImmichClient::from_env().expect("read stub address from environment");
    let response = client.search_smart("sunset").expect("run smart search");
    let mut picker = Picker::new();
    picker.add_response(&response);
    let got: Vec<String> = picker.selection().to_vec();
    assert_eq!(got, vec!["asset-s1".to_string()], "smart picker ids");
}

#[test]
fn verify_metadata_guard() {
    let client = ImmichClient::from_env().expect("read stub address from environment");
    let response = client
        .search_metadata("photo-0.jpg")
        .expect("run metadata search");
    let mut picker = Picker::new();
    picker.add_response(&response);
    let got: Vec<String> = picker.selection().to_vec();
    assert_eq!(got, vec!["asset-m1".to_string()], "metadata picker ids");
}
