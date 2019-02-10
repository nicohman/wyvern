use args::Command::Connect;
use args::Connect::*;
use gog::gog::connect::ConnectGameStatus::*;
use gog::gog::connect::*;
use gog::ErrorKind::*;
pub fn parse_args(gog: gog::Gog, args: ::args::Wyvern) -> gog::Gog {
    let uid: i64 = gog.get_user_data().unwrap().user_id.parse().unwrap();
    info!("Getting GOG Connect steam account");
    let linked = gog.connect_account(uid);
    if linked.is_err() {
        error!("You don't have a steam account linked to GOG! Go to https://www.gog.com/connect to link one.");
        return gog;
    } else {
        info!("Scanning for Connect games");
        gog.connect_scan(uid).unwrap();
    }
    match args.command {
        Connect(ListConnect { claim, quiet, json }) => {
            let status = gog.connect_status(uid);
            if status.is_ok() {
                let mut items = status.unwrap().items;
                let mut count_hidden = 0;
                let games: Vec<(String, ConnectGame)> = items
                    .into_iter()
                    .filter_map(|x| {
                        if !claim || x.1.status == READY_TO_LINK {
                            info!("Getting details for connect game");
                            let details = gog.product(vec![x.1.id], vec![]);
                            if details.is_ok() {
                                return Some((details.unwrap()[0].title.clone(), x.1));
                            }
                        }
                        count_hidden += 1;
                        return None;
                    })
                    .collect();
                if json {
                    let uct = ConnectList { games: games };
                    println!(
                        "{}",
                        serde_json::to_string(&uct)
                            .expect("Couldn't convert connect games list to string")
                    );
                } else {
                    for connect in games {
                        println!("{} - {:?}", connect.0, connect.1.status);
                    }
                }
                if !quiet {
                    println!("{} items not shown due to options", count_hidden);
                }
            } else {
                let err = status.err().unwrap();
                match err.kind() {
                    NotAvailable => error!("No GOG Connect games are available."),
                    _ => error!("{}", err),
                };
            }
        }
        Connect(ClaimAll) => {
            gog.connect_claim(uid).unwrap();
            println!("Claimed all available games");
        }
        _ => error!("Tell someone about this, because it should not be happening"),
    };
    gog
}
#[derive(Serialize, Debug)]
struct ConnectList {
    games: Vec<(String, ConnectGame)>,
}
