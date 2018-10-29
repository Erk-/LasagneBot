#[macro_use] extern crate log;
extern crate env_logger;

#[macro_use] extern crate serenity;

extern crate kankyo;
extern crate chrono;
extern crate rand;

use serenity::CACHE;
use serenity::framework::StandardFramework;
use serenity::framework::standard::{HelpBehaviour, help_commands};
use serenity::model::event::ResumedEvent;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::http;

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
             .lacking_permissions(HelpBehaviour::Hide)
             .striked_commands_tip(None)
             .command_not_found_text("Command not found {}")
             .lacking_role(HelpBehaviour::Hide)
             .wrong_channel(HelpBehaviour::Hide)
        })
        .group("Commands", |g| g
            .command("today", |c| c.cmd(today)
                .desc("todays comic"))
            .command("about", |c| c.cmd(about))
            .command("invite", |c| c.cmd(invite))
            .command("tomorrow", |c| c.cmd(tomorrow))
            .command("yesterday", |c| c.cmd(yesterday))   
            .command("date", |c| c.cmd(other_day)
                .num_args(1)
                .desc("comic from a specific date")
                .usage("yyyy-mm-dd"))
            .command("random", |c| c.cmd(random)
                .desc("random comic"))
            .command("stats", |c| c.cmd(stats)).owners_only(true)
        ));

    if let Err(why) = client.start() {
        error!("Client error: {:?}", why);
    }
}

command!(yesterday(_ctx, msg, _args) {
    let utc: DateTime<Utc> = Utc::now();
    let date: NaiveDate = NaiveDate::from_ymd(utc.year(), utc.month(), utc.day()).pred();
    let _ = match garfield_url(date) {
        Some(url) => msg.channel_id.send_message(|m| m
        .embed(|e| {
            e
                .author(|a| {a.name("Garfield.com").url("https://garfield.com")})
                .title(format!("Garfield: {}-{}-{}", date.year(), date.month(), date.day()))
                .url(format!("https://garfield.com/comic/{}/{}/{}", date.year(), date.month(), date.day()))
                .thumbnail("https://cdn.discordapp.com/attachments/381880193700069377/506066660839653386/favicon.png")
                .image(url.as_str())
                .colour((214,135,23))
        })),
        None => msg.channel_id.say("Invalid date."),
    };
});

command!(tomorrow(_ctx, msg, _args) {
    let mut rng = rand::thread_rng();
    let time_travel: [&str; 3] = ["2017-08-07", "2015-01-08", "1998-11-14"];
    let range = Range::new(0,3);
    let comic_date = range.ind_sample(&mut rng);
    let utc = match NaiveDate::parse_from_str(time_travel[comic_date], "%Y-%m-%d") {
        Ok(day) => day,
        Err(why) => {
            warn!("Error: {}, input: {}", why, comic_date);
            let _ = msg.channel_id.say("Invalid input.");
            return Ok(())
        },
    };
    let _ = match garfield_url(utc) {
        Some(url) => msg.channel_id.send_message(|m| m
        .embed(|e| {
            e
                .author(|a| {a.name("Garfield.com").url("https://garfield.com")})
                .title(format!("Garfield: {}-{}-{}", utc.year(), utc.month(), utc.day()))
                .url(format!("https://garfield.com/comic/{}/{}/{}", utc.year(), utc.month(), utc.day()))
                .thumbnail("https://cdn.discordapp.com/attachments/381880193700069377/506066660839653386/favicon.png")
                .image(url.as_str())
                .colour((214,135,23))
        })),
        None => msg.channel_id.say("Invalid date. (date should be between 1978-06-19 and today.)"),
    };
});

command!(invite(_ctx, msg, _args) {
    if let Err(why) =
        msg.channel_id.say("Invite the bot to your server: <https://discordapp.com/oauth2/authorize?client_id=404364579645292564&scope=bot>
") {
        warn!("Error sending message: {:?}", why);
    }
});

command!(stats(_ctx, msg, _args) {
    let guilds = {
        let cache = CACHE.read();
        cache.guilds.clone()
    };
    if let Err(why) = msg.channel_id.say(format!("The bot is in {} servers.", guilds.len())) {
        warn!("Error sending message: {:?}", why);
    }
    info!("The bot is in the following guilds:");
    for g in guilds {
        let pg = g.0.to_partial_guild()?;
        info!("{}", pg.name);
    }
});

command!(about(_ctx, msg, _args) {
    if let Err(why) = msg.channel_id.say("This is a garfield comic bot.") {
        warn!("Error sending message: {:?}", why);
    }
});

fn garfield_url(date: NaiveDate) -> Option<String> {
    //1978-06-19
    let now: DateTime<Utc> = Utc::now();
    let tday: NaiveDate = NaiveDate::from_ymd(now.year(), now.month(), now.day());
    if date > NaiveDate::from_ymd(1978, 6, 18) && date <= tday
    {
        Some(format!("https://d1ejxu6vysztl5.cloudfront.net/comics/garfield/{}/{}-{:02}-{:02}.gif?format=png",
                     date.year(),
                     date.year(),
                     date.month(),
                     date.day()))
    } else {
        None
    }
}

command!(today(_ctx, msg, _args) {
    let utc: DateTime<Utc> = Utc::now();
    let date: NaiveDate = NaiveDate::from_ymd(utc.year(), utc.month(), utc.day());
    let _ = match garfield_url(date) {
        Some(url) => msg.channel_id.send_message(|m| m
        .embed(|e| {
            e
                .author(|a| {a.name("Garfield.com").url("https://garfield.com")})
                .title(format!("Garfield: {}-{}-{}", date.year(), date.month(), date.day()))
                .url(format!("https://garfield.com/comic/{}/{}/{}", date.year(), date.month(), date.day()))
                .thumbnail("https://cdn.discordapp.com/attachments/381880193700069377/506066660839653386/favicon.png")
                .image(url.as_str())
                .colour((214,135,23))
        })),
        None => msg.channel_id.say("Invalid date."),
    };
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
            e
                .author(|a| {a.name("Garfield.com").url("https://garfield.com")})
                .title(format!("Garfield: {}-{}-{}", utc.year(), utc.month(), utc.day()))
                .url(format!("https://garfield.com/comic/{}/{}/{}", utc.year(), utc.month(), utc.day()))
                .thumbnail("https://cdn.discordapp.com/attachments/381880193700069377/506066660839653386/favicon.png")
                .image(url.as_str())
                .colour((214,135,23))
        })),
        None => msg.channel_id.say("Invalid date. (date should be between 1978-06-19 and today.)"),
    };
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
    let cmonth: usize = utc.month() as usize;
    let cday: usize = utc.day() as usize;

    let r0 = Range::new(1978, cyear+1);
    let r1 = Range::new(6, 12+1);
    let r2 = Range::new(1, cmonth+1);
    let r3 = Range::new(1, 12+1);
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
                6 => Range::new(19, 30+1).ind_sample(& mut rng),
                _ => Range::new(1, get_month_len(month) + 1).ind_sample(& mut rng),
            }
            },
       year if year == cyear => Range::new(1, cday + 1).ind_sample(& mut rng),
        _ => Range::new(1, get_month_len(month) + 1).ind_sample(& mut rng),
    };

    let date: NaiveDate = NaiveDate::from_ymd(year as i32, month as u32, day as u32);
    let _ = match garfield_url(date) {
        Some(url) => msg.channel_id.send_message(|m| m
        .embed(|e| {
            e
                .author(|a| {a.name("Garfield.com").url("https://garfield.com")})
                .title(format!("Garfield: {}-{}-{}", date.year(), date.month(), date.day()))
                .url(format!("https://garfield.com/comic/{}/{}/{}", date.year(), date.month(), date.day()))
                .thumbnail("https://cdn.discordapp.com/attachments/381880193700069377/506066660839653386/favicon.png")
                .image(url.as_str())
                .colour((214,135,23))
        })),
        None => msg.channel_id.say("Invalid date."),
    };
});
