//new
#[cfg(feature = "server")]
use reqwest;
#[cfg(feature = "server")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use std::env;

#[cfg(feature = "server")]
use dioxus::prelude::ServerFnError;

// Error types for better error handling
#[derive(Debug, thiserror::Error)]
#[cfg(feature = "server")]
pub enum EmailError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Environment variable error: {0}")]
    EnvError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("API error: {message}")]
    ApiError { message: String },
}

// Implement From<EmailError> for ServerFnError to allow ? operator
#[cfg(feature = "server")]
impl From<EmailError> for ServerFnError {
    fn from(err: EmailError) -> Self {
        ServerFnError::new(err.to_string())
    }
}

// Email data types for different templates
#[derive(Debug, Clone, Serialize)]
#[cfg(feature = "server")]
pub enum EmailType {
    SendOtp {
        otp_code: String,
    },
    OrderConfirmation {
        order_id: String,
        order_ref: String,
        order_date: String,
    },
    OrderConfirmationWithBackorder {
        order_id: String,
        order_ref: String,
        order_date: String,
    },
    TrackingConfirmation {
        order_id: String,
        order_ref: String,
        tracking_url: String,
    },
    PreOrderTrackingConfirmation {
        order_id: String,
        order_ref: String,
        tracking_url: String,
    },
    ExpressDispatchConfirmation {
        order_id: String,
        order_ref: String,
    },
    ExpressPreOrderDispatchConfirmation {
        order_id: String,
        order_ref: String,
    },
    ExpressTrackingConfirmation {
        order_id: String,
        order_ref: String,
        tracking_url: String,
    },
    ExpressPreOrderTrackingConfirmation {
        order_id: String,
        order_ref: String,
        tracking_url: String,
    },
    ExpiredOrder,
}

#[cfg(feature = "server")]
impl EmailType {
    // Returns the template ID for each email type
    pub fn template_id(&self) -> u32 {
        match self {
            EmailType::SendOtp { .. } => 5,
            EmailType::OrderConfirmation { .. } => 6,
            EmailType::OrderConfirmationWithBackorder { .. } => 14,
            EmailType::TrackingConfirmation { .. } => 7,
            EmailType::PreOrderTrackingConfirmation { .. } => 11,
            EmailType::ExpressDispatchConfirmation { .. } => 8,
            EmailType::ExpressPreOrderDispatchConfirmation { .. } => 13,
            EmailType::ExpressTrackingConfirmation { .. } => 10,
            EmailType::ExpressPreOrderTrackingConfirmation { .. } => 12,
            EmailType::ExpiredOrder => 9,
        }
    }

    // Returns the data payload for the template
    pub fn data(&self) -> Option<serde_json::Value> {
        match self {
            EmailType::SendOtp { otp_code } => Some(serde_json::json!({
                "otp_code": otp_code
            })),
            EmailType::OrderConfirmation {
                order_id,
                order_ref,
                order_date,
            } => Some(serde_json::json!({
                "order_id": order_id,
                "order_ref": order_ref,
                "order_date": order_date
            })),
            EmailType::OrderConfirmationWithBackorder {
                order_id,
                order_ref,
                order_date,
            } => Some(serde_json::json!({
                "order_id": order_id,
                "order_ref": order_ref,
                "order_date": order_date
            })),
            EmailType::TrackingConfirmation {
                order_id,
                order_ref,
                tracking_url,
            } => Some(serde_json::json!({
                "order_id": order_id,
                "order_ref": order_ref,
                "tracking_url": tracking_url
            })),
            EmailType::PreOrderTrackingConfirmation {
                order_id,
                order_ref,
                tracking_url,
            } => Some(serde_json::json!({
                "order_id": order_id,
                "order_ref": order_ref,
                "tracking_url": tracking_url
            })),
            EmailType::ExpressDispatchConfirmation {
                order_id,
                order_ref,
            } => Some(serde_json::json!({
                "order_id": order_id,
                "order_ref": order_ref
            })),
            EmailType::ExpressPreOrderDispatchConfirmation {
                order_id,
                order_ref,
            } => Some(serde_json::json!({
                "order_id": order_id,
                "order_ref": order_ref
            })),
            EmailType::ExpressTrackingConfirmation {
                order_id,
                order_ref,
                tracking_url,
            } => Some(serde_json::json!({
                "order_id": order_id,
                "order_ref": order_ref,
                "tracking_url": tracking_url
            })),
            EmailType::ExpressPreOrderTrackingConfirmation {
                order_id,
                order_ref,
                tracking_url,
            } => Some(serde_json::json!({
                "order_id": order_id,
                "order_ref": order_ref,
                "tracking_url": tracking_url
            })),
            EmailType::ExpiredOrder => None,
        }
    }
}

// Request payload for the Listmonk API
#[derive(Debug, Serialize)]
#[cfg(feature = "server")]
struct TransactionalEmailRequest {
    subscriber_email: String,
    template_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    content_type: String,
}

// Response from the Listmonk API
#[derive(Debug, Deserialize)]
#[cfg(feature = "server")]
struct ListmonkResponse {
    data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[cfg(feature = "server")]
struct ListmonkErrorResponse {
    message: String,
}

// Email service configuration
#[cfg(feature = "server")]
pub struct EmailService {
    client: reqwest::Client,
    api_url: String,
    auth_token: String,
}

// Subscriber creation request
#[derive(Debug, Serialize)]
#[cfg(feature = "server")]
struct CreateSubscriberRequest {
    email: String,
    name: String,
    status: String,
    lists: Vec<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attribs: Option<serde_json::Value>,
}

// List membership modification request
#[derive(Debug, Serialize)]
#[cfg(feature = "server")]
struct ModifyListMembershipRequest {
    ids: Vec<u32>,
    action: String,
    target_list_ids: Vec<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

// Subscriber list info
#[derive(Debug, Deserialize)]
#[cfg(feature = "server")]
struct SubscriberList {
    id: u32,
    subscription_status: String,
}

// Subscriber response
#[derive(Debug, Deserialize)]
#[cfg(feature = "server")]
struct Subscriber {
    id: u32,
    email: String,
    name: String,
    status: String,
    lists: Vec<SubscriberList>,
}

#[derive(Debug, Deserialize)]
#[cfg(feature = "server")]
struct SubscriberResponse {
    data: Subscriber,
}

#[derive(Debug, Deserialize)]
#[cfg(feature = "server")]
struct SubscriberSearchResponse {
    data: SubscriberSearchData,
}

#[derive(Debug, Deserialize)]
#[cfg(feature = "server")]
struct SubscriberSearchData {
    results: Vec<Subscriber>,
}

#[cfg(feature = "server")]
impl EmailService {
    const EMAIL_LIST_ID: u32 = 1;

    // Initialize the email service with environment variables
    pub fn new() -> Result<Self, EmailError> {
        let api_url = env::var("LISTMONK_API_URL").map_err(|_| {
            EmailError::EnvError("LISTMONK_API_URL environment variable must be set.".to_string())
        })?;

        let private_key = env::var("LISTMONK_PRIVATE_KEY").map_err(|_| {
            EmailError::EnvError(
                "LISTMONK_PRIVATE_KEY environment variable must be set.".to_string(),
            )
        })?;

        let client = reqwest::Client::new();

        Ok(Self {
            client,
            api_url,
            auth_token: private_key,
        })
    }

    // Search for a subscriber by email
    async fn find_subscriber_by_email(
        &self,
        email: &str,
    ) -> Result<Option<Subscriber>, EmailError> {
        let url = format!("{}/api/subscribers", self.api_url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("token {}", self.auth_token))
            .query(&[("query", format!("subscribers.email = '{}'", email))])
            .send()
            .await?;

        if response.status().is_success() {
            let search_response: SubscriberSearchResponse = response.json().await?;
            Ok(search_response.data.results.into_iter().next())
        } else if response.status().as_u16() == 404 {
            Ok(None)
        } else {
            let error_response: ListmonkErrorResponse =
                response
                    .json()
                    .await
                    .unwrap_or_else(|_| ListmonkErrorResponse {
                        message: "Unknown API error".to_string(),
                    });

            Err(EmailError::ApiError {
                message: error_response.message,
            })
        }
    }

    // Create a new subscriber
    async fn create_subscriber(
        &self,
        email: &str,
        name: &str,
        add_email_list: bool,
    ) -> Result<Subscriber, EmailError> {
        let lists = if add_email_list {
            vec![Self::EMAIL_LIST_ID]
        } else {
            vec![]
        };

        let request_payload = CreateSubscriberRequest {
            email: email.to_string(),
            name: name.to_string(),
            status: "enabled".to_string(),
            lists,
            attribs: None,
        };

        let url = format!("{}/api/subscribers", self.api_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("token {}", self.auth_token))
            .header("Content-Type", "application/json")
            .json(&request_payload)
            .send()
            .await?;

        if response.status().is_success() {
            let subscriber_response: SubscriberResponse = response.json().await?;
            Ok(subscriber_response.data)
        } else {
            let error_response: ListmonkErrorResponse =
                response
                    .json()
                    .await
                    .unwrap_or_else(|_| ListmonkErrorResponse {
                        message: "Unknown API error".to_string(),
                    });

            Err(EmailError::ApiError {
                message: error_response.message,
            })
        }
    }

    // Update subscriber list membership
    async fn update_subscriber_list_membership(
        &self,
        subscriber_id: u32,
        add_email_list: bool,
        is_currently_subscribed: bool,
    ) -> Result<(), EmailError> {
        // Only make API call if we need to change the subscription status
        if add_email_list == is_currently_subscribed {
            return Ok(());
        }

        let action = if add_email_list { "add" } else { "remove" };
        let status = if add_email_list {
            Some("confirmed".to_string())
        } else {
            None
        };

        let request_payload = ModifyListMembershipRequest {
            ids: vec![subscriber_id],
            action: action.to_string(),
            target_list_ids: vec![Self::EMAIL_LIST_ID],
            status,
        };

        let url = format!("{}/api/subscribers/lists", self.api_url);

        let response = self
            .client
            .put(&url)
            .header("Authorization", format!("token {}", self.auth_token))
            .header("Content-Type", "application/json")
            .json(&request_payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_response: ListmonkErrorResponse =
                response
                    .json()
                    .await
                    .unwrap_or_else(|_| ListmonkErrorResponse {
                        message: "Unknown API error".to_string(),
                    });

            return Err(EmailError::ApiError {
                message: error_response.message,
            });
        }

        Ok(())
    }

    // Ensure subscriber exists and has correct list membership
    async fn ensure_subscriber(
        &self,
        email: &str,
        name: &str,
        add_email_list: bool,
    ) -> Result<(), EmailError> {
        match self.find_subscriber_by_email(email).await? {
            Some(subscriber) => {
                // Check if subscriber is currently subscribed to the email list
                let is_currently_subscribed = subscriber
                    .lists
                    .iter()
                    .any(|list| list.id == Self::EMAIL_LIST_ID);

                // Update list membership if needed
                self.update_subscriber_list_membership(
                    subscriber.id,
                    add_email_list,
                    is_currently_subscribed,
                )
                .await?;
            }
            None => {
                // Create new subscriber with appropriate list membership
                self.create_subscriber(email, name, add_email_list).await?;
            }
        }

        Ok(())
    }

    // Send a transactional email (internal method)
    async fn send_transactional_email(
        &self,
        recipient_email: &str,
        email_type: EmailType,
    ) -> Result<(), EmailError> {
        let request_payload = TransactionalEmailRequest {
            subscriber_email: recipient_email.to_string(),
            template_id: email_type.template_id(),
            data: email_type.data(),
            content_type: "html".to_string(),
        };

        let url = format!("{}/api/tx", self.api_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("token {}", self.auth_token))
            .header("Content-Type", "application/json")
            .json(&request_payload)
            .send()
            .await?;

        if response.status().is_success() {
            // Parse successful response
            let _: ListmonkResponse = response.json().await?;
            Ok(())
        } else {
            // Parse error response
            let error_response: ListmonkErrorResponse =
                response
                    .json()
                    .await
                    .unwrap_or_else(|_| ListmonkErrorResponse {
                        message: "Unknown API error".to_string(),
                    });

            Err(EmailError::ApiError {
                message: error_response.message,
            })
        }
    }

    // Main public method to send email with automatic subscriber management
    pub async fn send_email(
        &self,
        recipient_email: &str,
        recipient_name: &str,
        email_type: EmailType,
        add_email_list: bool,
    ) -> Result<(), EmailError> {
        // Ensure subscriber exists and has correct list membership
        self.ensure_subscriber(recipient_email, recipient_name, add_email_list)
            .await?;

        // Send the email
        self.send_transactional_email(recipient_email, email_type)
            .await
    }
}

// Convenience functions for specific email types
#[cfg(feature = "server")]
impl EmailService {
    pub async fn send_otp(
        &self,
        recipient_email: &str,
        recipient_name: &str,
        otp_code: String,
    ) -> Result<(), EmailError> {
        let email_type = EmailType::SendOtp { otp_code };
        // OTP emails are always sent without adding to email list
        self.send_email(recipient_email, recipient_name, email_type, false)
            .await
    }

    pub async fn send_order_confirmation(
        &self,
        recipient_email: &str,
        recipient_name: &str,
        order_id: String,
        order_ref: String,
        order_date: String,
        add_email_list: bool,
    ) -> Result<(), EmailError> {
        let email_type = EmailType::OrderConfirmation {
            order_id,
            order_ref,
            order_date,
        };
        self.send_email(recipient_email, recipient_name, email_type, add_email_list)
            .await
    }

    pub async fn send_tracking_confirmation(
        &self,
        recipient_email: &str,
        recipient_name: &str,
        order_id: String,
        order_ref: String,
        tracking_url: String,
        add_email_list: bool,
    ) -> Result<(), EmailError> {
        let email_type = EmailType::TrackingConfirmation {
            order_id,
            order_ref,
            tracking_url,
        };
        self.send_email(recipient_email, recipient_name, email_type, add_email_list)
            .await
    }

    pub async fn send_express_dispatch_confirmation(
        &self,
        recipient_email: &str,
        recipient_name: &str,
        order_id: String,
        order_ref: String,
        add_email_list: bool,
    ) -> Result<(), EmailError> {
        let email_type = EmailType::ExpressDispatchConfirmation {
            order_id,
            order_ref,
        };
        self.send_email(recipient_email, recipient_name, email_type, add_email_list)
            .await
    }

    pub async fn send_expired_order_notification(
        &self,
        recipient_email: &str,
        recipient_name: &str,
        add_email_list: bool,
    ) -> Result<(), EmailError> {
        let email_type = EmailType::ExpiredOrder;
        self.send_email(recipient_email, recipient_name, email_type, add_email_list)
            .await
    }
}

pub fn okay_domains() -> Vec<&'static str> {
    let top_email_domains: Vec<&str> = vec![
        "gmail.com",
        "yahoo.com",
        "hotmail.com",
        "outlook.com",
        "aol.com",
        "icloud.com",
        "live.com",
        "msn.com",
        "yahoo.co.uk",
        "googlemail.com",
        "me.com",
        "mac.com",
        "yandex.ru",
        "mail.ru",
        "qq.com",
        "163.com",
        "126.com",
        "sina.com",
        "sohu.com",
        "yahoo.co.jp",
        "naver.com",
        "daum.net",
        "hanmail.net",
        "yahoo.de",
        "web.de",
        "gmx.de",
        "t-online.de",
        "orange.fr",
        "laposte.net",
        "free.fr",
        "yahoo.fr",
        "libero.it",
        "alice.it",
        "tin.it",
        "yahoo.it",
        "yahoo.es",
        "terra.es",
        "bol.com.br",
        "uol.com.br",
        "ig.com.br",
        "yahoo.com.br",
        "rediffmail.com",
        "yahoo.in",
        "protonmail.com",
        "proton.me",
        "fastmail.com",
        "zoho.com",
        "tutanota.com",
        "cock.li",
        "disroot.org",
        "gmx.com",
        "gmx.net",
        "gmx.at",
        "gmx.ch",
        "mail.com",
        "inbox.com",
        "rambler.ru",
        "list.ru",
        "bk.ru",
        "inbox.ru",
        "tut.by",
        "ukr.net",
        "bigmir.net",
        "i.ua",
        "meta.ua",
        "abv.bg",
        "dir.bg",
        "mail.bg",
        "atlas.sk",
        "azet.sk",
        "centrum.sk",
        "post.sk",
        "email.cz",
        "seznam.cz",
        "centrum.cz",
        "volny.cz",
        "tiscali.it",
        "virgilio.it",
        "yahoo.ca",
        "sympatico.ca",
        "rogers.com",
        "bell.net",
        "videotron.ca",
        "shaw.ca",
        "telus.net",
        "yahoo.com.au",
        "bigpond.com",
        "optusnet.com.au",
        "iinet.net.au",
        "tpg.com.au",
        "yahoo.co.nz",
        "xtra.co.nz",
        "slingshot.co.nz",
        "yahoo.com.mx",
        "terra.com.mx",
        "prodigy.net.mx",
        "hotmail.es",
        "hotmail.de",
        "hotmail.fr",
        "hotmail.it",
        "hotmail.co.uk",
        "live.de",
        "live.fr",
        "live.it",
        "live.co.uk",
        "btinternet.com",
        "sky.com",
        "virgin.net",
        "talk21.com",
        "ntlworld.com",
        "wanadoo.fr",
        "sfr.fr",
        "neuf.fr",
        "club-internet.fr",
        "aliceadsl.fr",
        "yahoo.com.sg",
        "singnet.com.sg",
        "starhub.net.sg",
        "pacific.net.sg",
        "yahoo.com.hk",
        "netvigator.com",
        "hknet.com",
        "biznetvigator.com",
        "yahoo.com.tw",
        "seed.net.tw",
        "hinet.net",
        "so-net.net.tw",
        "pchome.com.tw",
        "etang.com",
        "yeah.net",
        "tom.com",
        "21cn.com",
        "188.com",
        "citiz.net",
        "excite.com",
        "lycos.com",
        "juno.com",
        "earthlink.net",
        "comcast.net",
        "verizon.net",
        "sbcglobal.net",
        "bellsouth.net",
        "charter.net",
        "cox.net",
        "roadrunner.com",
        "att.net",
        "optonline.net",
        "windstream.net",
        "frontier.com",
        "centurylink.net",
        "rocketmail.com",
        "ymail.com",
        "aim.com",
        "netscape.net",
        "usa.net",
        "email.com",
        "rediff.com",
        "sify.com",
    ];
    top_email_domains
}
