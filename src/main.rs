use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::sync::Mutex;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::fs::File;
use std::io::Write;
use chrono::Local;

// Structs
#[derive(Serialize, Deserialize, Clone)]
struct Client {
    id: Uuid,
    client_name: String,
    birth_date: String,
    document_number: String,
    country: String,
    balance: Decimal,
}

#[derive(Serialize, Deserialize)]
struct NewClientRequest {
    client_name: String,
    birth_date: String,
    document_number: String,
    country: String,
}
#[derive(Serialize, Deserialize)]
struct CreditTransactionRequest {
    id: Uuid,
    amount: Decimal,
}

#[derive(Serialize, Deserialize)]
struct DebitTransactionRequest {
    id: Uuid,
    debit_amount: Decimal,
}

#[derive(Serialize)]
struct BalanceResponse {
    balance: Decimal,
}

#[derive(Serialize)]
struct StoredBalancesResponse {
    filename: String,
}

#[derive(Serialize, Deserialize)]
struct ClientBalanceResponse {
    id: Uuid,
    client_name: String,
    birth_date: String,
    document_number: String,
    country: String,
    balance: Decimal,
}

#[derive(Deserialize)]
struct UserIdParams {
    user_id: Uuid,
}

// State
struct AppState {
    clients: Mutex<Vec<Client>>,
    counter: Mutex<u32>,
}

// Handlers
async fn new_client(data: web::Data<AppState>, req: web::Json<NewClientRequest>) -> impl Responder {
    let mut clients = data.clients.lock().unwrap();

    if req.document_number.is_empty() {
        return HttpResponse::BadRequest().body("El documento es inconsistente");
    }

    if clients.iter().any(|client| client.document_number == req.document_number) {
        return HttpResponse::BadRequest().body("El documento ya se encuentra registrado");
    }

    let new_client = Client {
        id: Uuid::new_v4(),
        client_name: req.client_name.clone(),
        birth_date: req.birth_date.clone(),
        document_number: req.document_number.clone(),
        country: req.country.clone(),
        balance: dec!(0.0),
    };

    clients.push(new_client.clone());

    HttpResponse::Ok().json(new_client.id)
}

async fn new_credit_transaction(data: web::Data<AppState>, req: web::Json<CreditTransactionRequest>) -> impl Responder {
    let mut clients = data.clients.lock().unwrap();

    if req.amount <= dec!(0) {
        return HttpResponse::BadRequest().body("Revise el monto ingresado");
    }

    let client_index = match clients.iter().position(|client| client.id == req.id) {
        Some(index) => index,
        None => return HttpResponse::NotFound().body("No se encontro el cliente"),
    };

    let client = &mut clients[client_index];
    client.balance += req.amount;

    HttpResponse::Ok().json(BalanceResponse {
        balance: client.balance,
    })
}

async fn new_debit_transaction(data: web::Data<AppState>, req: web::Json<DebitTransactionRequest>) -> impl Responder {
    let mut clients = data.clients.lock().unwrap();

    if req.debit_amount <= dec!(0) {
        return HttpResponse::BadRequest().body("Revise el monto ingresado");
    }

    let client_index = match clients.iter().position(|client| client.id == req.id) {
        Some(index) => index,
        None => return HttpResponse::NotFound().body("No se encontro el cliente"),
    };

    let client = &mut clients[client_index];

    if req.debit_amount > client.balance {
        return HttpResponse::BadRequest().body("Fondos insuficientes");
    }

    client.balance -= req.debit_amount;

    HttpResponse::Ok().json(BalanceResponse {
        balance: client.balance,
    })
}

async fn store_balances(data: web::Data<AppState>) -> impl Responder {
    let mut clients = data.clients.lock().unwrap();

    let timestamp = Local::now();
    let date_str = timestamp.format("%d%m%Y").to_string();

    let mut counter = data.counter.lock().unwrap();
    *counter += 1;
    let filename = format!("{}_{}.DAT", date_str, *counter);

    let mut client_balances = String::new();
    for client in clients.iter_mut() {
        client_balances.push_str(&format!("{} {}\n", client.id, client.balance));
        client.balance = dec!(0.0); 
    }

    let mut file = File::create(&filename).unwrap();
    file.write_all(client_balances.as_bytes()).unwrap();

    HttpResponse::Ok().json(StoredBalancesResponse { filename })
}

async fn client_balance(data: web::Data<AppState>, params: web::Query<UserIdParams>) -> impl Responder {
    let clients = data.clients.lock().unwrap();

    let client = clients.iter().find(|client| client.id == params.user_id);

    match client {
        Some(client) => {
            let response = ClientBalanceResponse {
                id: client.id,
                client_name: client.client_name.clone(),
                birth_date: client.birth_date.clone(),
                document_number: client.document_number.clone(),
                country: client.country.clone(),
                balance: client.balance,
            };
            HttpResponse::Ok().json(response)
        },
        None => HttpResponse::NotFound().body("Cliente no encontrado"),
    }
}

// Main function
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state = web::Data::new(AppState {
        clients: Mutex::new(Vec::new()),
        counter: Mutex::new(0),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/new_client", web::post().to(new_client))
            .route("/new_credit_transaction", web::post().to(new_credit_transaction))
            .route("/new_debit_transaction", web::post().to(new_debit_transaction))
            .route("/store_balances", web::post().to(store_balances))
            .route("/client_balance", web::get().to(client_balance))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
