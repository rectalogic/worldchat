use std::time::Duration;

use pkarr::dns::rdata::RData;

static NAME: &str = "worldchat_test_abedgghr";

#[tokio::test]
async fn test_publish() {
    let client = pkarr::Client::builder().build().unwrap();
    let keypair = pkarr::Keypair::random();
    let endpoint1 = "endpoint1";
    let endpoint2 = "endpoint2";
    let endpoint3 = "endpoint3";

    dump_records(&client, &keypair, "initial records").await;

    publish_endpoint(&client, &keypair, endpoint1.try_into().unwrap(), 10).await;
    tokio::time::sleep(Duration::from_secs(5)).await;
    dump_records(&client, &keypair, "published endpoint1").await;

    publish_endpoint(&client, &keypair, endpoint2.try_into().unwrap(), 7).await;
    tokio::time::sleep(Duration::from_secs(5)).await;
    dump_records(&client, &keypair, "published endpoint2").await;

    publish_endpoint(&client, &keypair, endpoint3.try_into().unwrap(), 7).await;
    tokio::time::sleep(Duration::from_secs(5)).await;
    dump_records(&client, &keypair, "published endpoint3").await;

    tokio::time::sleep(Duration::from_secs(10)).await;
    dump_records(&client, &keypair, "final records").await;
}

async fn publish_endpoint(
    client: &pkarr::Client,
    keypair: &pkarr::Keypair,
    endpoint_name: pkarr::dns::Name<'_>,
    ttl: u32,
) {
    let mut builder = pkarr::SignedPacket::builder();
    let (builder, cas) =
        if let Some(most_recent) = client.resolve_most_recent(&keypair.public_key()).await {
            for record in most_recent.fresh_resource_records(NAME) {
                match record.rdata {
                    RData::CNAME(ref cname) if cname.0 != endpoint_name => {
                        match record.ttl.overflowing_sub(most_recent.elapsed()) {
                            (_, true) => {}
                            (ttl, false) => {
                                let mut record = record.clone();
                                record.ttl = ttl;
                                builder = builder.record(record);
                            }
                        }
                    }
                    _ => {}
                }
            }
            (builder, Some(most_recent.timestamp()))
        } else {
            (builder, None)
        };

    let signed_packet = builder
        .cname(NAME.try_into().unwrap(), endpoint_name, ttl)
        .sign(keypair)
        .unwrap();

    client.publish(&signed_packet, cas).await.unwrap();
}

async fn dump_records(client: &pkarr::Client, keypair: &pkarr::Keypair, message: &str) {
    println!("{}", message);
    if let Some(most_recent) = client.resolve_most_recent(&keypair.public_key()).await {
        for record in most_recent.fresh_resource_records(NAME) {
            match record.rdata {
                RData::CNAME(ref cname) => {
                    println!("{} ttl {}", cname.0, record.ttl);
                }
                _ => {
                    println!("{:?} ttl {}", record.rdata, record.ttl);
                }
            }
        }
    } else {
        println!("No records found");
    }
}
