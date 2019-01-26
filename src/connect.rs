use args::Connect::*;
use args::Wyvern::Connect;
use gog::gog::connect::ConnectGameStatus::*;
use gog::gog::connect::*;
use gog::ErrorKind::*;
pub fn parse_args(gog: gog::Gog, args: ::args::Wyvern) {
    let uid: i64 = gog.get_user_data().unwrap().user_id.parse().unwrap();
    info!("Getting GOG Connect steam account");
    let linked = gog.connect_account(uid);
    if linked.is_err() {
        error!("You don't have a steam account linked to GOG! Go to https://www.gog.com/connect to link one.");
        return;
    } else {
        info!("Scanning for Connect games");
        gog.connect_scan(uid).unwrap();
    }
    match args {
        Connect(ListConnect {
            claim,
            quiet,
            verbose,
        }) => {
            verbose
                .setup_env_logger("wyvern")
                .expect("Couldn't set up logger");
            info!("Getting GOG Connect status");
            let status = gog.connect_status(uid);
            if status.is_ok() {
                let mut items = status.unwrap().items;
                let left_over: Vec<(String, ConnectGame)> = items
                    .into_iter()
                    .filter_map(|x| {
                        if !claim || x.1.status == READY_TO_LINK {
                            info!("Getting details for connect game");
                            let details = gog.product(vec![x.1.id], vec![]);
                            if details.is_ok() {
                                println!("{} - {:?}", details.unwrap()[0].title, x.1.status);
                                return None;
                            }
                        }
                        return Some(x);
                    })
                    .collect();
                if !quiet {
                    println!("{} items not shown due to options", left_over.len());
                }
            } else {
                let err = status.err().unwrap();
                match err.kind() {
                    NotAvailable => error!("No GOG Connect games are available."),
                    _ => error!("{}", err),
                };
            }
        }
        Connect(ClaimAll { verbose }) => {
            verbose
                .setup_env_logger("wyvern")
                .expect("Couldn't set up logger");
            gog.connect_claim(uid).unwrap();
            println!("Claimed all available games");
        }
        _ => error!("Tell someone about this, because it should not be happening"),
    }
}
