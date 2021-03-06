#![cfg(all(feature = "elastictranscoder", feature = "s3"))]

extern crate env_logger;
#[macro_use]
extern crate log;
extern crate rand;
extern crate rusoto_core;
extern crate rusoto_elastictranscoder;
extern crate rusoto_s3;

use std::clone::Clone;
use std::ops::{Deref, DerefMut};

use rand::Rng;
use rusoto_core::Region;
use rusoto_elastictranscoder::{Ets, EtsClient};
use rusoto_s3::{CreateBucketRequest, DeleteBucketRequest, S3Client, S3};

const AWS_ETS_WEB_PRESET_ID: &'static str = "1351620000001-100070";
const AWS_ETS_WEB_PRESET_NAME: &'static str = "System preset: Web";
const AWS_REGION: Region = Region::UsEast1;
const AWS_SERVICE_RANDOM_SUFFIX_LENGTH: usize = 20;

struct TestEtsClient {
    region: Region,

    client: EtsClient,

    s3_client: Option<S3Client>,
    input_bucket: Option<String>,
    output_bucket: Option<String>,
}

impl TestEtsClient {
    /// Creates a new `EtsClient` for a test.
    fn new(region: Region) -> TestEtsClient {
        TestEtsClient {
            region: region.clone(),
            client: EtsClient::new(region),
            s3_client: None,
            input_bucket: None,
            output_bucket: None,
        }
    }

    fn create_s3_client(&mut self) {
        self.s3_client = Some(S3Client::new(self.region.clone()));
    }

    fn create_bucket(&mut self) -> String {
        let bucket_name = generate_unique_name("ets-bucket-1");

        let create_bucket_req = CreateBucketRequest {
            bucket: bucket_name.to_owned(),
            ..Default::default()
        };

        let result = self
            .s3_client
            .as_ref()
            .unwrap()
            .create_bucket(create_bucket_req)
            .sync();

        let mut location = result.unwrap().location.unwrap();
        // A `Location` is identical to a `BucketName` except that it has a
        // forward slash prepended to it, so we need to remove it.
        location.remove(0);
        info!("Created S3 bucket: {}", location);
        location
    }
}

impl Deref for TestEtsClient {
    type Target = EtsClient;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for TestEtsClient {
    fn deref_mut<'a>(&'a mut self) -> &'a mut EtsClient {
        &mut self.client
    }
}

impl Drop for TestEtsClient {
    fn drop(&mut self) {
        self.s3_client.take().map(|s3_client| {
            self.input_bucket.take().map(|bucket| {
                let delete_bucket_req = DeleteBucketRequest {
                    bucket: bucket.to_owned(),
                    ..Default::default()
                };

                match s3_client.delete_bucket(delete_bucket_req).sync() {
                    Ok(_) => info!("Deleted S3 bucket: {}", bucket),
                    Err(e) => error!("Failed to delete S3 bucket: {}", e),
                };
            });
            self.output_bucket.take().map(|bucket| {
                let delete_bucket_req = DeleteBucketRequest {
                    bucket: bucket.to_owned(),
                    ..Default::default()
                };

                match s3_client.delete_bucket(delete_bucket_req).sync() {
                    Ok(_) => info!("Deleted S3 bucket: {}", bucket),
                    Err(e) => error!("Failed to delete S3 bucket: {}", e),
                };
            });
        });
    }
}

// TODO: once Rust has proper support for testing frameworks, this code will
// need to be refactored so that it is only called once, instead of per test
// case.
fn initialize() {
    let _ = env_logger::try_init();
}

fn create_client() -> TestEtsClient {
    TestEtsClient::new(AWS_REGION)
}

/// Generates a random name for an AWS service by appending a random sequence of
/// ASCII characters to the specified prefix.
/// Keeps it lower case to work with S3 requirements as of 3/1/2018.
fn generate_unique_name(prefix: &str) -> String {
    format!(
        "{}-{}",
        prefix,
        rand::thread_rng()
            .gen_ascii_chars()
            .take(AWS_SERVICE_RANDOM_SUFFIX_LENGTH)
            .collect::<String>()
    )
    .to_lowercase()
}

#[test]
#[should_panic(expected = "Role cannot be blank")]
fn create_pipeline_without_arn() {
    use rusoto_elastictranscoder::CreatePipelineRequest;

    initialize();

    let mut client = create_client();
    client.create_s3_client();
    client.input_bucket = Some(client.create_bucket());
    client.output_bucket = Some(client.create_bucket());

    let request = CreatePipelineRequest {
        input_bucket: client.input_bucket.as_ref().cloned().unwrap(),
        output_bucket: client.output_bucket.as_ref().cloned(),
        ..CreatePipelineRequest::default()
    };
    let response = client.create_pipeline(request).sync();

    response.unwrap();
}

#[test]
fn create_preset() {
    use rusoto_elastictranscoder::{
        AudioCodecOptions, AudioParameters, CreatePresetRequest, DeletePresetRequest,
    };

    initialize();

    let client = create_client();

    let name = generate_unique_name("ets-preset-1");
    let request = CreatePresetRequest {
        audio: Some(AudioParameters {
            channels: Some("2".to_owned()),
            codec: Some("flac".to_owned()),
            codec_options: Some(AudioCodecOptions {
                bit_depth: Some("24".to_owned()),
                ..AudioCodecOptions::default()
            }),
            sample_rate: Some("96000".to_owned()),
            ..AudioParameters::default()
        }),
        container: "flac".to_owned(),
        description: Some("This is an example FLAC preset".to_owned()),
        name: name.clone(),
        ..CreatePresetRequest::default()
    };
    let response = client.create_preset(request).sync();

    assert!(response.is_ok());

    let response = response.unwrap();

    assert!(response.preset.is_some());

    let preset = response.preset.unwrap();

    assert_eq!(preset.container, Some("flac".to_owned()));
    assert_eq!(
        preset.description,
        Some("This is an example FLAC preset".to_owned())
    );
    assert_eq!(preset.name, Some(name));
    assert!(preset.id.is_some());

    let id = preset.id.unwrap();

    info!("Created preset with id: {:?}", &id);

    // Cleanup

    let request = DeletePresetRequest { id: id };
    client.delete_preset(request).sync().ok();
}

#[test]
fn delete_preset() {
    use rusoto_elastictranscoder::{
        AudioCodecOptions, AudioParameters, CreatePresetRequest, DeletePresetRequest,
    };

    initialize();

    let client = create_client();

    let name = generate_unique_name("ets-preset-1");
    let request = CreatePresetRequest {
        audio: Some(AudioParameters {
            channels: Some("2".to_owned()),
            codec: Some("flac".to_owned()),
            codec_options: Some(AudioCodecOptions {
                bit_depth: Some("24".to_owned()),
                ..AudioCodecOptions::default()
            }),
            sample_rate: Some("96000".to_owned()),
            ..AudioParameters::default()
        }),
        container: "flac".to_owned(),
        description: Some("This is an example FLAC preset".to_owned()),
        name: name.clone(),
        ..CreatePresetRequest::default()
    };
    let response = client.create_preset(request).sync().unwrap();
    let preset = response.preset.unwrap();
    let id = preset.id.unwrap();

    let request = DeletePresetRequest { id: id.clone() };
    let response = client.delete_preset(request).sync();

    assert!(response.is_ok());
    info!("Deleted preset with id: {:?}", &id);
}

#[test]
fn list_jobs_by_status() {
    use rusoto_elastictranscoder::ListJobsByStatusRequest;

    initialize();

    let client = create_client();

    let status = "Submitted".to_owned();
    let request = ListJobsByStatusRequest {
        status: status.clone(),
        ..ListJobsByStatusRequest::default()
    };
    let response = client.list_jobs_by_status(request).sync();

    assert!(response.is_ok());

    let response = response.unwrap();

    info!(
        "Got list of jobs with status \"{}\": {:?}",
        &status, response.jobs
    );
}

#[test]
fn list_pipelines() {
    use rusoto_elastictranscoder::ListPipelinesRequest;

    initialize();

    let client = create_client();

    let request = ListPipelinesRequest::default();
    let response = client.list_pipelines(request).sync();

    assert!(response.is_ok());

    let response = response.unwrap();

    info!("Got list of pipelines: {:?}", response.pipelines);
}

#[test]
fn list_presets() {
    use rusoto_elastictranscoder::ListPresetsRequest;

    initialize();
    let client = create_client();

    let request = ListPresetsRequest::default();
    let response = client.list_presets(request).sync();
    assert!(response.is_ok());

    let response = response.unwrap();
    assert!(response.presets.is_some());

    let presets = response.presets.unwrap();
    info!("Got list of presets.");
    for preset in presets.iter() {
        info!("Preset: {:?}", preset.name);
    }

    let web_preset = presets
        .iter()
        .filter(|x| x.id == Some(AWS_ETS_WEB_PRESET_ID.to_owned()))
        .next();

    let found_preset = match web_preset {
        Some(w) => w.clone(),
        None => {
            // get the next page
            assert!(response.next_page_token.is_some());
            let page_two_request = ListPresetsRequest {
                page_token: response.next_page_token,
                ..Default::default()
            };

            let page_two_response = client.list_presets(page_two_request).sync().unwrap();
            let presets_pg_2 = page_two_response.presets.unwrap();
            let web_preset = presets_pg_2
                .iter()
                .filter(|x| x.id == Some(AWS_ETS_WEB_PRESET_ID.to_owned()))
                .next();
            web_preset.unwrap().clone()
        }
    };

    assert_eq!(found_preset.id, Some(AWS_ETS_WEB_PRESET_ID.to_owned()));
    assert_eq!(found_preset.name, Some(AWS_ETS_WEB_PRESET_NAME.to_owned()));
}

#[test]
fn read_preset() {
    use rusoto_elastictranscoder::ReadPresetRequest;

    initialize();

    let client = create_client();

    let request = ReadPresetRequest {
        id: AWS_ETS_WEB_PRESET_ID.to_owned(),
    };
    let response = client.read_preset(request).sync();

    assert!(response.is_ok());

    let response = response.unwrap();

    assert!(response.preset.is_some());

    let preset = response.preset.unwrap();
    info!("Got preset: {:?}", preset.name);

    assert_eq!(preset.id, Some(AWS_ETS_WEB_PRESET_ID.to_owned()));
    assert_eq!(preset.name, Some(AWS_ETS_WEB_PRESET_NAME.to_owned()));
}
