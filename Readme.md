# P2P Recipe App - Refactoring Checklist

## Critical Fixes

### Error Handling
- [ ] Replace `panic!("Unknown command")` with `error!()` logging
- [ ] Replace `panic!("Invalid id...")` in `handle_publish_recipes` with error return
- [ ] Replace all `.expect()` calls in user input paths with proper `Result` handling
- [ ] Add graceful handling for channel receiver closure in event loop
- [ ] Wrap `response.expect("Response exists")` in proper error handling

### File I/O Race Conditions
- [ ] Add `Arc<Mutex<Recipes>>` or `Arc<RwLock<Recipes>>` for thread-safe recipe access
- [ ] Refactor `read_local_recipes()` to use shared state instead of file reads
- [ ] Refactor `write_local_recipes()` to use shared state with periodic persistence
- [ ] Add file locking mechanism if keeping file-based approach
- [ ] Implement atomic file writes (write to temp file, then rename)

### Input Validation
- [ ] Validate recipe name length (max 100 chars)
- [ ] Validate ingredients length (max 1000 chars)
- [ ] Validate instructions length (max 2000 chars)
- [ ] Check for empty/whitespace-only inputs in `handle_create_recipes`
- [ ] Sanitize inputs to prevent injection attacks
- [ ] Add maximum recipe count limit per peer (prevent DoS)
- [ ] Validate peer ID format in `ListMode::One`

### Network Security
- [ ] Migrate from Floodsub to Gossipsub (Floodsub is deprecated)
- [ ] Implement message signing using peer identity
- [ ] Add message verification on receipt
- [ ] Implement rate limiting for incoming requests
- [ ] Add message size limits (prevent memory exhaustion)
- [ ] Consider encrypting recipe content (optional, based on requirements)

## High Priority

### Command Parsing
- [ ] Create `Command` enum with variants for each command type
- [ ] Implement `FromStr` trait for `Command` enum
- [ ] Replace string prefix matching with enum-based dispatch
- [ ] Add command help/usage system
- [ ] Handle unknown commands gracefully with suggestions

### Application Lifecycle
- [ ] Add Ctrl+C signal handler for graceful shutdown
- [ ] Implement proper cleanup in shutdown (close swarm, save data)
- [ ] Add startup checks (file permissions, port availability)
- [ ] Create initialization sequence with error recovery
- [ ] Add "quit" or "exit" command

### Logging & Debugging
- [ ] Log responses not meant for this peer (with debug level)
- [ ] Add structured logging with different levels (trace, debug, info, warn, error)
- [ ] Log all network events with timestamps
- [ ] Add metrics collection (messages sent/received, peers discovered)
- [ ] Improve bootstrap connection logging (track actual connection establishment)

### User Feedback
- [ ] Add confirmation messages for all operations
- [ ] Show progress indicators for network operations
- [ ] Display recipe count after create/publish operations
- [ ] Add "recipe not found" messages with helpful suggestions
- [ ] Show peer count in `ls p` command

## Medium Priority

### Code Organization
- [ ] Split into multiple files (commands.rs, network.rs, storage.rs, types.rs)
- [ ] Create dedicated `CommandHandler` struct
- [ ] Extract network setup into separate function
- [ ] Move all constants to a config module
- [ ] Create `RecipeStore` abstraction for storage layer

### Type Safety
- [ ] Create NewType wrappers for `RecipeId`, `PeerId` strings
- [ ] Add builder pattern for `Recipe` struct
- [ ] Use `NonZeroUsize` for recipe IDs
- [ ] Add validation methods to structs (e.g., `Recipe::validate()`)
- [ ] Make `Recipe` fields private with getters

### Error Types
- [ ] Create custom error enum (e.g., `RecipeError`)
- [ ] Implement `std::error::Error` for custom errors
- [ ] Add context to errors using libraries like `anyhow` or `thiserror`
- [ ] Remove generic `Box<dyn Error>` in favor of specific types
- [ ] Add error recovery strategies

### Testing
- [ ] Add unit tests for `create_new_recipe`
- [ ] Add unit tests for `publish_recipe`
- [ ] Add unit tests for command parsing
- [ ] Add integration tests for network behavior
- [ ] Mock file I/O for testing
- [ ] Add property-based tests for recipe validation
- [ ] Test concurrent access scenarios

## Low Priority

### Storage
- [ ] Consider SQLite instead of JSON file
- [ ] Add recipe versioning (track edits)
- [ ] Implement recipe deletion command
- [ ] Add recipe search functionality
- [ ] Support recipe categories/tags
- [ ] Add backup/export functionality

### Network Features
- [ ] Implement peer reputation system
- [ ] Add peer blacklisting/whitelisting
- [ ] Cache received recipes to reduce network traffic
- [ ] Add recipe update propagation
- [ ] Implement request timeout handling
- [ ] Add NAT traversal support

### User Experience
- [ ] Add interactive mode with readline support
- [ ] Implement recipe editing command
- [ ] Add recipe import/export (JSON, CSV)
- [ ] Create CLI with proper argument parsing (using `clap`)
- [ ] Add colored output for better readability
- [ ] Show recipe preview in list view

### Documentation
- [ ] Add rustdoc comments to all public items
- [ ] Create README with setup instructions
- [ ] Document network protocol/message formats
- [ ] Add architecture diagram
- [ ] Create troubleshooting guide
- [ ] Add examples directory with use cases

### Performance
- [ ] Profile and optimize hot paths
- [ ] Implement lazy loading for large recipe lists
- [ ] Add caching layer for frequently accessed recipes
- [ ] Optimize serialization (consider bincode vs JSON)
- [ ] Batch file writes to reduce I/O

### Configuration
- [ ] Move hardcoded values to config file
- [ ] Support environment variables for all settings
- [ ] Add command-line arguments for common options
- [ ] Create default config with sensible values
- [ ] Add config validation on startup

---