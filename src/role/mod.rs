/// Role Module: Because every AI needs a personality, even if it's a sarcastic one.
/// "Roles are like hats - you can wear different ones, but they all make you look slightly ridiculous."
/// 
/// This module provides functionality for managing AI roles, audiences, presets, and styles.
/// Because apparently, we need to tell AI how to behave, like training a very sarcastic parrot.
/// 
/// # Example
/// ```rust
/// use role::{Role, Audience, Preset};
/// 
/// let role = Role::translator(Some(Audience::Scientist), Some(Preset::Questions));
/// println!("Now your AI will translate like a scientist who's had too much coffee!");
/// ```

pub mod role;
pub mod audience;
pub mod preset;
pub mod style;

pub use role::Role;
pub use audience::Audience;
pub use preset::Preset;
pub use style::Style; 