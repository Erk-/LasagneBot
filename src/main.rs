#[macro_use] extern crate log;
extern crate env_logger;

#[macro_use] extern crate serenity;

extern crate kankyo;
extern crate chrono;
extern crate rand;

use serenity::framework::StandardFramework;
use serenity::model::event::ResumedEvent;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::framework::standard::{HelpBehaviour, help_commands};
//use serenity::model::prelude::*;
use serenity::http;

//use rand::*;
//use rand::Rng;
use rand::distributions::{IndependentSample, Range};

use std::collections::HashSet;
use std::env;

use chrono::prelude::*;

struct Handler;

impl EventHandler for Handler {
    fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }

    fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

fn main() {
    // This will load the environment variables located at `./.env`, relative to
    // the CWD. See `./.env.example` for an example on how to structure this.
    kankyo::load().expect("Failed to load .env file");
    env_logger::init();

    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");

    let mut client = Client::new(&token, Handler).expect("Err creating client");

    let owners = match http::get_current_application_info() {
        Ok(info) => {
            let mut set = HashSet::new();
            set.insert(info.owner.id);

            set
        },
        Err(why) => panic!("Couldn't get application info: {:?}", why),
    };

    client.with_framework(StandardFramework::new()
        .configure(|c| c
            .owners(owners)
            .on_mention(true)
            .prefix("="))
        .customised_help(help_commands::with_embeds, |c| {
            c.individual_command_tip("If you want more information about a command pass it as an argument to help.")
             .lacking_permissions(HelpBehaviour::Nothing)
             .striked_commands_tip(None)
             .command_not_found_text("Command not found {}")
             .lacking_role(HelpBehaviour::Nothing)
             .wrong_channel(HelpBehaviour::Nothing)
        })
        .group("Commands", |g| g
            .command("today", |c| c.cmd(today)
                .desc("todays comic"))
            .command("about", |c| c.cmd(about))
            .command("date", |c| c.cmd(other_day)
                .num_args(1)
                .desc("comic from a specific date")
                .usage("yyyy-mm-dd"))
            .command("random", |c| c.cmd(random)
                .desc("random comic"))));

    if let Err(why) = client.start() {
        error!("Client error: {:?}", why);
    }
}

command!(about(_ctx, msg, _args) {
    if let Err(why) = msg.channel_id.say("This is a garfield comic bot.") {
        warn!("Error sending message: {:?}", why);
    }
});

fn garfield_url(date: NaiveDate) -> Option<String> {
    //1978-06-19
    let now: DateTime<Utc> = Utc::now();
    if (date.year() < 1978 || 
       (date.year() == 1978 && date.month() < 6 && date.day() < 19)) || 
       (date.year() >= now.year() && date.month() >= now.month() && date.day() > now.day() + 1) {
        None
    } else {
        Some(format!("https://d1ejxu6vysztl5.cloudfront.net/comics/garfield/{}/{}-{:02}-{:02}.gif?format=png", date.year(), date.year(), date.month(), date.day()))
    }
}

command!(today(_ctx, msg, _args) {
    let utc: DateTime<Utc> = Utc::now();
    let date: NaiveDate = NaiveDate::from_ymd(utc.year(), utc.month(), utc.day());
    let _ = match garfield_url(date) {
        Some(url) => msg.channel_id.send_message(|m| m
        .embed(|e| {
            let mut e = e
                .image(url.as_str());
                e
        })),
        None => msg.channel_id.say("Invalid date."),
    };
    ()
});

command!(help(_ctx, msg, _args) {
    let _ = msg.channel_id.send_message(|m| m
        .embed(|e| {
            let mut e = e
                .title("Garfield help")
                .field("Help", "=today, todays comic\n=date yyyy-mm-dd, get a comic\n=random, random comic\n=info, this message", false);
            e
        }));
    ()
});

command!(other_day(_ctx, msg, args) {
    let date = args.single::<String>().unwrap();
    let utc = match NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
        Ok(day) => day,
        Err(why) => {
            warn!("Error: {}, input: {}", why, date);
            let _ = msg.channel_id.say("Invalid input.");
            return Ok(())
        },
    };

    let _ = match garfield_url(utc) {
        Some(url) => msg.channel_id.send_message(|m| m
        .embed(|e| {
            let mut e = e
                .image(url.as_str());
                e
        })),
        None => msg.channel_id.say("Invalid date. (date should be between 1978-06-19 and today.)"),
    };
    ()
});

fn get_month_len(month: usize) -> usize {
    match month {
        1 => 31,
        2 => 28,
        3 => 31,
        4 => 30,
        5 => 31,
        6 => 30,
        7 => 31,
        8 => 31,
        9 => 30,
        10 => 31,
        11 => 30,
        12 => 31,
        _ => 31,
    }
}

command!(random(_ctx, msg, _args) {
    let utc: DateTime<Utc> = Utc::now();
    let cyear: usize = utc.year() as usize;
    let cmonth: usize = match utc.month() as usize {
        1 => 2,
        _ => utc.month() as usize,
    };
    let cday: usize = match utc.day() as usize {
        1 => 2,
        _ => utc.day() as usize,
    };

    let r0 = Range::new(1978, cyear);
    let r1 = Range::new(6, 12);
    let r2 = Range::new(1, cmonth);
    let r3 = Range::new(1, 12);
// TODO rewite to use rand range usize!!!
    let mut rng = rand::thread_rng();
    let year: usize = r0.ind_sample(& mut rng);
    let month: usize = match year {
        1978 => r1.ind_sample(& mut rng),
        year if year == cyear => r2.ind_sample(&mut rng),
        _ => r3.ind_sample(& mut rng),
    };
    let day: usize = match year {
        1978 => {
            match month {
                6 => Range::new(19, 30).ind_sample(& mut rng),
                _ => Range::new(1, (get_month_len(month))).ind_sample(& mut rng),
            }
            },
        year if year == cyear => Range::new(1, cday).ind_sample(& mut rng),
        _ => Range::new(1, (get_month_len(month))).ind_sample(& mut rng),
    };

    let date: NaiveDate = NaiveDate::from_ymd(year as i32, month as u32, day as u32);
    let _ = match garfield_url(date) {
        Some(url) => msg.channel_id.send_message(|m| m
        .embed(|e| {
            let mut e = e
                .title(format!("{}-{}-{}", year, month, day))
                .image(url.as_str());
                e
        })),
        None => msg.channel_id.say("Invalid date."),
    };
    ()
});