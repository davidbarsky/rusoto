#![cfg(feature = "storagegateway")]

extern crate rusoto_core;
extern crate rusoto_storagegateway;

use rusoto_core::Region;
use rusoto_storagegateway::{ListGatewaysInput, StorageGateway, StorageGatewayClient};

#[test]
fn should_list_gateways() {
    let client = StorageGatewayClient::new(Region::UsEast1);
    let request = ListGatewaysInput::default();

    client.list_gateways(request).sync().unwrap();
}
