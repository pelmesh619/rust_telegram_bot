use grammers_client::{Client, InputMessage, SignInError};
use grammers_client::client::auth::InvocationError;
use grammers_client::types::Message;
use regex::Regex;
use crate::utils;

pub const SESSION_FILE: &str = "userbot.session";

pub fn ask_input_from_user() -> String {
    loop {
        let mut result = String::new();
        match std::io::stdin().read_line(&mut result) {
            Ok(_) => break result.trim().to_string(),
            Err(e) => println!("Some error raised: {}", e)
        }
    }
}

pub async fn sign_in(client: &Client, ) -> crate::Result {
    if client.is_authorized().await? {
        return Ok(());
    }
    println!("Signing in...");

    let (token_or_phone, is_token) = loop {
        println!("Enter your phone number or bot token");
        let token_or_phone = ask_input_from_user();

        let phone_regex = Regex::new(r"^[+]?[(]?[0-9]{3}[)]?[-\s.]?[0-9]{3}[-\s.]?[0-9]{4,6}$").unwrap();
        let token_regex = Regex::new(r"^\d+:[A-Za-z0-9_-]+$").unwrap();

        if token_regex.is_match(&*token_or_phone) {
            break (token_or_phone, true)
        } else if phone_regex.is_match(&*token_or_phone) {
            break (token_or_phone, false)
        } else {
            println!("This is not a phone number neither bot token. Try again.");
            continue
        }
    };

    if is_token {
        client.bot_sign_in(&token_or_phone).await?;
    } else {
        let token = client.request_login_code(&*token_or_phone).await?;
        println!("The code has been successfully requested. Enter the Telegram code:");
        let code = ask_input_from_user();

        let user = match client.sign_in(&token, &code).await {
            Ok(user) => user,
            Err(SignInError::PasswordRequired(_token)) => {
                let hint = _token.hint().unwrap_or("None");
                println!("You have a two-factor authentication, please enter your password: \n(Hint - {})", hint);
                loop {
                    let password = ask_input_from_user();
                    break match client.check_password(_token.clone(), password).await {
                        Ok(t) => t,
                        Err(e) => {
                            println!("It seems to be some error raised: {}", e);
                            continue;
                        },
                    }
                }
            }
            Err(SignInError::SignUpRequired { terms_of_service: _tos }) =>
                panic!("It seems to be that this number does not have a Telegram account. Please sign up on other device and continue."),
            Err(err) => {
                println!("Failed to sign in as a user :(\n{}", err);
                return Err(err.into());
            }
        };
        println!("Signed in as {}!", user.first_name());
    }

    client.session().save_to_file(SESSION_FILE)?;
    return Ok(());
}

pub async fn send_message(client: &Client, chat: grammers_client::types::Chat, text: &str) -> Result<Message, InvocationError> {
    let (text, entities) = utils::parse_entities(&*text);
    fn f_<T: Into<InputMessage>>(m: T) -> InputMessage {
        return m.into();
    }
    let input_message = f_(text.clone());
    client.send_message(&chat, input_message.fmt_entities(entities)).await
}

pub async fn reply_to_message(message: &Message, text: &str) -> Result<Message, InvocationError> {
    let (text, entities) = utils::parse_entities(&*text);
    fn f_<T: Into<InputMessage>>(m: T) -> InputMessage {
        return m.into();
    }
    let input_message = f_(text.clone());
    message.reply(input_message.fmt_entities(entities)).await
}

pub async fn edit_message(message: &Message, text: &str) -> Result<(), InvocationError> {
    let (text, entities) = utils::parse_entities(&*text);
    fn f_<T: Into<InputMessage>>(m: T) -> InputMessage {
        return m.into();
    }
    let input_message = f_(text.clone());
    message.edit(input_message.fmt_entities(entities)).await
}

