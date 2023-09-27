use btleplug::api::bleuuid::uuid_from_u16;
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::Manager;
use futures::StreamExt;

use crate::bthome::Object;

mod bthome;

#[derive(Debug, PartialEq)]
pub struct Update {
    name: String,
    object: Object,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let manager = Manager::new().await?;

    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().next().expect("no adapters found");

    let mut events = central.events().await?;

    central.start_scan(ScanFilter::default()).await?;

    while let Some(event) = events.next().await {
        if let CentralEvent::ServiceDataAdvertisement { id, service_data } = event {
            if let Some(data) = service_data.get(&uuid_from_u16(0x181c)) {
                let peripherals = central.peripherals().await.unwrap();

                let Some(peripheral) = peripherals.iter().find(|p| p.id() == id) else {
                    eprintln!("got ad from unknown peripheral");
                    continue;
                };

                let Some(properties) = peripheral.properties().await.unwrap() else {
                    eprintln!("got ad from peripheral with no properties");
                    continue;
                };

                let Some(name) = properties.local_name else {
                    eprintln!("got ad from peripheral with no name");
                    continue;
                };

                let mut objects = bthome::decode(data.as_slice())
                    .await
                    .into_iter()
                    .filter(|obj| match obj {
                        Object::Temperature(_) | Object::Humidity(_) => true,
                        Object::Battery(_) | Object::Voltage(_) | Object::Power(_) => false,
                        Object::Rssi(_) => unreachable!(),
                    })
                    .map(|object| Update { name: name.clone(), object })
                    .collect::<Vec<_>>();

                if let Some(rssi) = properties.rssi {
                    objects.push(Update {
                        name: name.clone(),
                        object: Object::Rssi(rssi),
                    });
                }

                println!("{name} {objects:?}");
            }
        }
    }

    Ok(())
}
