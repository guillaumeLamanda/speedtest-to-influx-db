use chrono::{DateTime, Utc};
use influxdb::{Client, InfluxDbWriteable};
use speedtest_rs::speedtest::{
    get_best_server_based_on_latency, get_configuration, get_server_list_with_config,
    test_download_with_progress_and_config, test_upload_with_progress_and_config,
};
use std::env::args;
use std::io::Write;

fn print_dot() {
    print!(".");
    std::io::stdout().flush().unwrap();
}

fn get_influx_db_url_from_env() -> String {
    std::env::var("INFLUX_DB_URL").expect(
        "no influxdb url specified. Use first argument or INFLUX_DB_URL environment variable.",
    )
}

#[async_std::main]
async fn main() {
    let client_addr = args().nth(1).unwrap_or_else(get_influx_db_url_from_env);

    let client = Client::new(client_addr, "speedtests");
    match client.ping().await {
        Err(err) => {
            println!("Error pinging influxdb {}", err);
        }
        _ => (),
    };
    let mut config = get_configuration().expect("config error");
    let servers_config = get_server_list_with_config(&config).expect("server list error");
    let server_and_latency =
        get_best_server_based_on_latency(&servers_config.servers).expect("latency error");
    println!("latency: {}ms", server_and_latency.latency.as_millis());

    let download =
        test_download_with_progress_and_config(&server_and_latency.server, print_dot, &mut config)
            .expect("error while download measurement");
    println!("\ndownload: {}kbps", download.kbps());
    let upload =
        test_upload_with_progress_and_config(&server_and_latency.server, print_dot, &mut config)
            .expect("error while upload measurement");
    println!("\nupload: {}kbps", upload.kbps());

    #[derive(InfluxDbWriteable)]
    struct SpeedTestMeasure {
        time: DateTime<Utc>,
        latency: f64,
        upload: f64,
        download: f64,
    }

    let measure_to_write = SpeedTestMeasure {
        time: Utc::now(),
        latency: server_and_latency.latency.as_secs_f64(),
        download: download.bps_f64(),
        upload: upload.bps_f64(),
    };
    match client
        .query(&measure_to_write.into_query("speedtest"))
        .await
    {
        Err(e) => {
            println!("Error ! {}", e);
            panic!("error while writing result");
        }
        _ => (),
    }
}
