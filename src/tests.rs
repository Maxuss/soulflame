use crate::net_io::{PacketRead, PacketWrite};
use std::io::Cursor;
use tokio::test;
use tokio::time::Instant;

const PROTO_VERSION: u32 = 759;

#[test]
async fn packet_io() -> anyhow::Result<()> {
    let vi = "Test".to_string();
    let mut buffer = vec![];
    vi.pack_write(&mut buffer, PROTO_VERSION).await?;
    println!("{:?}", buffer);
    let got = String::pack_read(&mut Cursor::new(&buffer), PROTO_VERSION).await?;
    println!("{:?} - {:?}", vi, got);
    assert_eq!(vi, got);
    Ok(())
}

#[test]
async fn packet_vec() -> anyhow::Result<()> {
    let start = Instant::now();

    let vec = vec![
        "string1".to_string(),
        "string2".to_string(),
        "string3".to_string(),
    ];
    let mut buffer = vec![];
    vec.pack_write(&mut buffer, PROTO_VERSION).await?;
    println!("{:?}", buffer);
    let got = Vec::<String>::pack_read(&mut Cursor::new(&buffer), PROTO_VERSION).await?;
    println!("{:?} - {:?}", vec, got);

    report_time(start);
    Ok(())
}

#[test]
async fn packet_test() -> anyhow::Result<()> {
    let start = Instant::now();

    report_time(start);
    Ok(())
}

fn report_time(i: Instant) {
    let dur = Instant::now() - i;

    println!("Took {}mcs", dur.as_micros())
}
