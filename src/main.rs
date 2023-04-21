use axum::body::StreamBody;
use axum::{routing::get, Router};
use axum_login::memory_store::MemoryStore;
use axum_login::secrecy::SecretVec;
use axum_login::{AuthLayer, AuthUser};
use axum_sessions::extractors::WritableSession;
use axum_sessions::SessionLayer;
use futures_util::{stream, Stream};

#[derive(Debug, Default, Clone)]
struct User {
	id: i64,
}

impl AuthUser<i64> for User {
	fn get_id(&self) -> i64 {
		self.id
	}

	fn get_password_hash(&self) -> SecretVec<u8> {
		SecretVec::new(vec![])
	}
}

type AuthContext = axum_login::extractors::AuthContext<i64, User, MemoryStore<i64, User>>;

#[tokio::main]
async fn main() {
	let secret = [0u8; 64];

	let session_store = axum_sessions::async_session::MemoryStore::new();
	let session_layer = SessionLayer::new(session_store, &secret).with_secure(false);

	let user_store = MemoryStore::<i64, User>::new(&Default::default());
	let auth_layer = AuthLayer::new(user_store, &secret);

	let app = Router::new()
		.route("/stream", get(route_streaming_example))
		.route("/login", get(route_streaming_login))
		.route("/logout", get(route_logout))
		.route("/session", get(route_session))
		.layer(auth_layer)
		.layer(session_layer);

	// run it with hyper on localhost:3000
	axum::Server::bind(&"0.0.0.0:3002".parse().unwrap())
		.serve(app.into_make_service())
		.await
		.unwrap();
}

// Works as normal, including with passing in a database extension and all kinds of things
async fn route_streaming_example() -> StreamBody<impl Stream<Item = std::io::Result<&'static str>>> {
	let stream = stream::once(async move {
		// performing some long operation
		tokio::time::sleep(std::time::Duration::from_secs(2)).await;
		Ok("done")
	});
	StreamBody::new(stream)
}

// Doesn't work, axum-login is blowing up
async fn route_streaming_login(mut auth: AuthContext) -> StreamBody<impl Stream<Item = std::io::Result<&'static str>>> {
	let stream = stream::once(async move {
		// performing some long operation
		tokio::time::sleep(std::time::Duration::from_secs(2)).await;
		// And need to access auth 'later' in the stream
		auth.login(&User { id: 1 }).await.unwrap();
		Ok("logged in")
	});
	StreamBody::new(stream)
}

// Accessing it not in a stream works fine though
async fn route_logout(mut auth: AuthContext) -> &'static str {
	auth.logout().await;
	"logged out"
}

// And interestingly discovered axum-sessions causes a freeze when the session is interacted with
async fn route_session(mut session: WritableSession) -> StreamBody<impl Stream<Item = std::io::Result<&'static str>>> {
	let stream = stream::once(async move {
		// performing some long operation
		tokio::time::sleep(std::time::Duration::from_secs(2)).await;
		// And need to access session 'later' in the stream
		session.insert("test", 42).unwrap();
		Ok("got session")
	});
	StreamBody::new(stream)
}
