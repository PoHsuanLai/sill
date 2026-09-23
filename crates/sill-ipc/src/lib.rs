//! The sill IPC protocol: newline-delimited JSON requests and responses, one per connection.

use serde as _;
use serde_json as _;
use thiserror as _;
