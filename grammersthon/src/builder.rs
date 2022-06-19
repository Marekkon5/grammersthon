use std::path::Path;
use crossterm::style::Attribute;
use grammers_client::{InitParams, Client, Config, SignInError};
use grammers_session::Session;
use tokio::io::{AsyncWriteExt, BufReader, AsyncBufReadExt};

use crate::Grammersthon;
use crate::error::GrammersthonError;

pub struct GrammersthonBuilder {
    api_id: i32,
    api_hash: String,
    bot_token: Option<String>,
    session: Session,
    phone: Option<String>,
    params: InitParams,
    interactive: bool,
    password_hint: bool,
    password: Option<String>
}

impl GrammersthonBuilder {
    /// Create new builder instance
    pub fn new(api_id: i32, api_hash: &str) -> GrammersthonBuilder {
        GrammersthonBuilder {
            api_id,
            api_hash: api_hash.to_string(),
            bot_token: None,
            session: Session::new(),
            phone: None,
            params: InitParams::default(),
            interactive: true,
            password_hint: false,
            password: None
        }
    }

    /// Set session parameter for client
    pub fn use_memory_session(mut self) -> Self {
        self.session = Session::new();
        self
    }

    /// Shorthand for setting the session client parameter from path
    /// Equivalent to: `.session(Session::load_file_or_create("session.session")?)`
    pub fn session_file(mut self, path: impl AsRef<Path>) -> Result<Self, GrammersthonError> {
        self.session = Session::load_file_or_create(path)?;
        Ok(self)
    }

    /// Login using bot token
    pub fn bot_token(mut self, token: &str) -> Self {
        self.bot_token = Some(token.to_string());
        self
    }

    /// Login using phone number
    pub fn phone(mut self, phone: &str) -> Self {
        self.phone = Some(phone.to_string());
        self
    }

    /// Set new client `InitParams`
    pub fn params(mut self, params: InitParams) -> Self {
        self.params = params;
        self
    }

    /// Enable interactive mode (prompt in terminal for missing fields)
    pub fn interactive(mut self, enabled: bool) -> Self {
        self.interactive = enabled;
        self
    }

    /// Wether to display password hint in interactive mode
    pub fn show_password_hint(mut self, show: bool) -> Self {
        self.password_hint = show;
        self
    }

    /// Set the password for logging in
    pub fn password(mut self, password: Option<&str>) -> Self {
        self.password = password.map(String::from);
        self
    }

    /// Prompt for a question in CLI
    async fn prompt(question: &str, hide: bool) -> Result<String, GrammersthonError> {
        let mut stdout = tokio::io::stdout();
        stdout.write_all(question.as_bytes()).await?;
        if hide {
            stdout.write_all(Attribute::Hidden.to_string().as_bytes()).await?;
        }
        stdout.flush().await?;

        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut output = String::new();
        reader.read_line(&mut output).await?;
        if hide {
            stdout.write_all(Attribute::NoHidden.to_string().as_bytes()).await?;
        }
        Ok(output.trim().to_string())
    }

    /// Build the client and try to connect
    pub async fn connect(mut self) -> Result<Grammersthon, GrammersthonError> {
        let mut client = Client::connect(Config {
            session: self.session,
            api_id: self.api_id,
            api_hash: self.api_hash.clone(),
            params: self.params,
        })
        .await?;

        if client.is_authorized().await? {
            return Grammersthon::from_client(client).await;
        }

        // Missing bot token and phone number
        if self.bot_token.is_none() && self.phone.is_none() {
            if !self.interactive {
                return Err(GrammersthonError::MissingParameters("bot_token or phone number"));
            }
            let answer = Self::prompt("Enter phone number or bot token: ", false).await?;
            if answer.contains(":") {
                self.bot_token = Some(answer);
            } else {
                self.phone = Some(answer);
            }
        }

        // Login using bot token
        if let Some(token) = self.bot_token {
            client.bot_sign_in(&token, self.api_id, &self.api_hash).await?;
            return Grammersthon::from_client(client).await;
        }

        // Unauthorized (can't prompt for code)
        if !self.interactive {
            return Err(GrammersthonError::MissingParameters("interactive (code prompt)"))
        }

        // Interactive user login
        let token = client.request_login_code(self.phone.as_ref().unwrap(), self.api_id, &self.api_hash).await?;
        let code = Self::prompt("Enter the code you received: ", false).await?;
        match client.sign_in(&token, &code).await {
            Ok(_) => Grammersthon::from_client(client).await,
            Err(SignInError::PasswordRequired(password_token)) => {
                // Try saved password
                if let Some(password) = &self.password {
                    match client.check_password(password_token, password).await {
                        Err(SignInError::InvalidPassword) => {
                            warn!("Invalid password!");
                            return Err(SignInError::InvalidPassword.into());
                        }
                        r => {
                            r?;
                            return Grammersthon::from_client(client).await;
                        }
                    };
                // Prompt for password
                } else {
                    let prompt = if self.password_hint && password_token.hint().is_some() {
                        format!("Enter your password (hint: {}) (hidden): ", password_token.hint().unwrap())
                    } else {
                        "Enter your password (hidden): ".to_string()
                    };
                    let answer = Self::prompt(&prompt, true).await?;
                    client.check_password(password_token, &answer).await?;
                    Grammersthon::from_client(client).await
                }
                
            }
            Err(e) => Err(e.into()),
        }
    }
}

