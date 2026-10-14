use serde::{Deserialize, Deserializer, Serialize};

const DEFAULT_BASE_URL: &str = "http://localhost:3030";

fn bool_from_int<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum BoolLike {
        Bool(bool),
        Int(i64),
    }
    Ok(match BoolLike::deserialize(deserializer)? {
        BoolLike::Bool(b) => b,
        BoolLike::Int(n) => n != 0,
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeathersPage<T> {
    pub data: Vec<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Addon {
    #[serde(rename = "addonId")]
    pub addon_id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Order {
    #[serde(rename = "orderId")]
    pub order_id: i64,
    #[serde(rename = "drinkId")]
    pub drink_id: i64,
    #[serde(rename = "milkId")]
    pub milk_id: Option<i64>,
    pub name: String,
    pub status: OrderStatus,
    #[serde(rename = "isHot", deserialize_with = "bool_from_int")]
    pub is_hot: bool,
    pub addons: Vec<Addon>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrderStatus {
    Ordered,
    Brewing,
    ReadyForPickup,
    Complete,
}

impl OrderStatus {
    pub fn label(self) -> &'static str {
        match self {
            OrderStatus::Ordered => "NEW",
            OrderStatus::Brewing => "BREWING",
            OrderStatus::ReadyForPickup => "READY",
            OrderStatus::Complete => "PICKED UP",
        }
    }

    pub fn next(self) -> Option<OrderStatus> {
        match self {
            OrderStatus::Ordered => Some(OrderStatus::Brewing),
            OrderStatus::Brewing => Some(OrderStatus::ReadyForPickup),
            OrderStatus::ReadyForPickup => Some(OrderStatus::Complete),
            OrderStatus::Complete => None,
        }
    }

    pub fn action_label(self) -> Option<&'static str> {
        match self {
            OrderStatus::Ordered => Some("Start brewing"),
            OrderStatus::Brewing => Some("Mark ready"),
            OrderStatus::ReadyForPickup => Some("Hand off"),
            OrderStatus::Complete => None,
        }
    }

    pub fn all() -> [OrderStatus; 4] {
        [
            OrderStatus::Ordered,
            OrderStatus::Brewing,
            OrderStatus::ReadyForPickup,
            OrderStatus::Complete,
        ]
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Drink {
    #[serde(rename = "drinkId")]
    pub drink_id: i64,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub temp: Option<String>,
    #[serde(default, deserialize_with = "bool_from_int")]
    pub featured: bool,
    #[serde(default, deserialize_with = "bool_from_int")]
    pub active: bool,
    #[serde(default)]
    pub position: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Bean {
    #[serde(rename = "beanId")]
    pub bean_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub roaster: Option<String>,
    pub notes: Option<String>,
    pub roast: Option<String>,
    #[serde(rename = "onDeck", default, deserialize_with = "bool_from_int")]
    pub on_deck: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Milk {
    #[serde(rename = "milkId")]
    pub milk_id: i64,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct NewOrder {
    #[serde(rename = "drinkId")]
    pub drink_id: i64,
    #[serde(rename = "milkId", skip_serializing_if = "Option::is_none")]
    pub milk_id: Option<i64>,
    pub name: String,
    #[serde(rename = "isHot")]
    pub is_hot: bool,
    #[serde(rename = "addonIds")]
    pub addon_ids: Vec<i64>,
}

#[derive(Debug, Serialize)]
struct StatusPatch {
    status: OrderStatus,
}

#[derive(Clone)]
pub struct Client {
    agent: ureq::Agent,
    base: String,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout(std::time::Duration::from_secs(5))
                .build(),
            base: std::env::var("BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string()),
        }
    }

    fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> color_eyre::Result<Vec<T>> {
        let url = format!("{}/{path}", self.base);
        let page: FeathersPage<T> = self.agent.get(&url).call()?.into_json()?;
        Ok(page.data)
    }

    pub fn fetch_orders(&self) -> color_eyre::Result<Vec<Order>> {
        self.get("buppers/orders?$sort[createdAt]=-1&$limit=100")
    }

    pub fn fetch_drinks(&self) -> color_eyre::Result<Vec<Drink>> {
        self.get("buppers/drinks?active=true&$sort[position]=1&$limit=100")
    }

    pub fn fetch_beans(&self) -> color_eyre::Result<Vec<Bean>> {
        self.get("buppers/beans?active=true&$limit=100")
    }

    pub fn fetch_milks(&self) -> color_eyre::Result<Vec<Milk>> {
        self.get("buppers/milks?active=true&$limit=100")
    }

    pub fn fetch_addons(&self) -> color_eyre::Result<Vec<Addon>> {
        self.get("buppers/addons?active=true&$limit=100")
    }

    pub fn create_order(&self, order: &NewOrder) -> color_eyre::Result<Order> {
        let url = format!("{}/buppers/orders", self.base);
        let created: Order = self.agent.post(&url).send_json(order)?.into_json()?;
        Ok(created)
    }

    pub fn advance_order(&self, order_id: i64, status: OrderStatus) -> color_eyre::Result<Order> {
        let url = format!("{}/buppers/orders/{order_id}", self.base);
        let patched: Order = self
            .agent
            .patch(&url)
            .send_json(StatusPatch { status })?
            .into_json()?;
        Ok(patched)
    }
}
