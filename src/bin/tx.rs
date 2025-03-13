#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    rng::Rng,
    timer::{systimer::SystemTimer, timg::TimerGroup},
};
use esp_println::println;
use esp_wifi::{
    esp_now::{EspNowManager, PeerInfo, BROADCAST_ADDRESS},
    init, EspWifiController,
};

// When you are okay with using a nightly compiler it's better to use https://docs.rs/static_cell/2.1.0/static_cell/macro.make_static.html
macro_rules! make_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);

    let esp_wifi_ctrl = &*make_static!(
        EspWifiController<'static>,
        init(
            timg0.timer0,
            Rng::new(peripherals.RNG),
            peripherals.RADIO_CLK,
        )
        .unwrap()
    );

    let wifi = peripherals.WIFI;
    let esp_now = esp_wifi::esp_now::EspNow::new(&esp_wifi_ctrl, wifi).unwrap();

    let systimer = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(systimer.alarm0);

    let (manager, _sender, mut receiver) = esp_now.split();
    let manager = make_static!(EspNowManager<'static>, manager);

    loop {
        let r = receiver.receive_async().await;
        println!("Received {:?}", r.data());
        if r.info.dst_address == BROADCAST_ADDRESS {
            if !manager.peer_exists(&r.info.src_address) {
                manager
                    .add_peer(PeerInfo {
                        peer_address: r.info.src_address,
                        lmk: None,
                        channel: None,
                        encrypt: false,
                    })
                    .unwrap();
                println!("Added peer {:?}", r.info.src_address);
            }
        }
    }
}
