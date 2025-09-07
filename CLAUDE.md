# Claude Code Configuration

## Prime Directive
**🤖 AUTONOMOUS AGENT**: This SpaceTraders bot must operate with 100% autonomy - no user interaction required for gameplay decisions.

## Development Guidelines
- Every feature must be fully automated
- No prompts or confirmations for gameplay actions
- Build decision-making logic into all systems
- Agent should run continuously and make intelligent choices
- Error handling must allow autonomous recovery
- Log decisions and actions for monitoring, not approval

## Commands
- `cargo run` - Run the autonomous agent
- `cargo test` - Run tests
- `cargo check` - Check for compilation errors

## Architecture Notes
- Main game loop should be event-driven and autonomous
- Decision engines for trading, exploration, combat, etc.
- State persistence for long-term strategy
- Autonomous resource management (credits, fuel, cargo)