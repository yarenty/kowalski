## Version

We use the latest Rust edition for new developments (2024 at time if writing). Legacy versions use the edition stated in their `Cargo.toml` file.

## Coding Conventions

* We do not use unsafe code
* Production code **must not** panic

## File Structure

The basic structure for all Rust source files should be like this:

```rust
//!
//! File header; see below
//!

/* --- uses ------------------------------------------------------------------------------------ */

use serde::Deserialize;
...

/* --- modules --------------------------------------------------------------------------------- */

mod thesaurus;
...

/* --- types ----------------------------------------------------------------------------------- */

///
/// Broker specific configuration options.
///
pub struct BrokerConfig {
  ...
}
...

/* --- constants -------------------------------------------------------------------------------- */

/** the version as defined in cargo.toml */
const VERSION: &str = env!("CARGO_PKG_VERSION");

/* --- start of code ---------------------------------------------------------------------------- */

///
/// Gives plumperquatsch a new name.
///
/// Since plumperquatsch is picky, this function will first ask him if he likes the new name.
/// If he does not, he might change the name to whatever he likes.
///
/// # Arguments
///  * `name` - the name to give to plumperquatsch
///
/// # Returns
///  * the new name of plumperquatsch

pub fn plumperquatsch(name: &str) -> String {
  ...
}

}
```

### Comments

As already stated in the generic coding conventions, comments are of paramount importance. See comments there for details.

Rust supports block and line comments; in general, we leave it up to you to choose the appropriate one.

#### Doc Comments

Almost all tools we came across so far have problems parsing block doc comments - at least if they span multiple lines. So doc comments for structs, functions etc. must use line comments; struct members may still be commented with block comments if they do not span multiple lines.
if in doubt, use line comments.

Example:

```rust
///
/// Handles expired sessions for the authorization context
///
pub struct ExpiredSessions{
  /** the authorization context that owns the sessions. */
  auth_context: Arc<RwLock<BBAuthContext>>,
  /** pass in the session clean-up interval - to allow for usage in testing etc. */
  session_cleanup_interval: Duration,
}
```

For functions, the doc comment must contain:

* Leading and trailing empty line
* Single line describing briefly what the function does
* Optional multiline description
* Arguments - mandatory
* Examples - if usage is not obvious, an example might help
* Panics - if the function might panic, list the conditions here; note that **production code must ever panic**
* Errors - if `Result<>` is returned and the Errors are not obvious

Sections in the doc comment must start with header 1 ("# Arguments" below) with no blank line.

Example doc comment:

````rust
///
/// Single line explanation of what the function does; do not repeat the function name.
///
/// Multiple lines for a more detailed description in Markdown syntax. Add
/// as many lines as you like.
///
/// # Arguments
///  * `ctx` - async-graphql context
///  * `patient_id` - ID of the patient that created the feedback; if not set, the
///     PATIENT-UUID HTTP header must be set
///  * `feedback` - feedback input data
///
/// # Panics
///  * If the file cannot be found
///
/// # Examples
///
/// ```
/// /* You can have rust code between fences inside the comments
///  * If you pass --test to `rustdoc`, it will even test it for you!
///  */
///  use doc::Person;
///  let person = Person::new("name");
/// ```
///
````

#### File Header Comments

All source files must contain a doc comment header. Start the comment with `//!` to mark it as module-level.

The header comment must contain:

* Single line short description of the functionality covered in the file
* Optional longer, multiline description
* Authors
* Copyright line

Example:

```rust
//!
//! Configuration parser for Jaros' broker.
//!
//! Prepares a configuration struct based on defaults and an optional TOML config file.
//!
//! Authors:
//!   Jaro <yarenty@gmail.com>
//!
//! Copyright (c) 2026 Jaro
//!
```

### Other Rust Specifics

#### Lifetime Specifiers

We think that lifetime specifiers are a pain; in general, we try to avoid them. So before you use them, make sure that alternatives (which usually mean cloning objects) are too expensive.

In other words, use lifetime specifiers only:

* If the otherwise cloned objects are large
* If the cloning has to happen frequently
