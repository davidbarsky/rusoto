#![cfg(feature = "glue")]

extern crate rusoto_core;
extern crate rusoto_glue;

use rusoto_core::Region;
use rusoto_glue::{GetDatabasesRequest, Glue, GlueClient};

#[test]
fn should_get_databases() {
    let client = GlueClient::new(Region::UsWest2);
    let request = GetDatabasesRequest::default();

    let result = client.get_databases(request).sync().unwrap();
    println!("{:#?}", result);
}
