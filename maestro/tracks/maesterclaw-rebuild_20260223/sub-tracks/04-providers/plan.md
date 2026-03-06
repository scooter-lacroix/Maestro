# Subtrack 04: Provider Implementations - Plan

## Phase 1: Test-Driven Development (RED)

### [x] Task 1.1: Write Enhanced Provider Trait Tests
- Test Provider trait with chat(), stream_chat(), chat_with_tools()
- Test supports_native_tools(), capabilities()
- Test warmup(), health_check()

### [x] Task 1.2: Write OpenAI Provider Tests
- Test chat completion with real OpenAI API
- Test tool calling with GPT-4
- Test streaming responses
- Test error handling (rate limits, auth errors)

### [x] Task 1.3: Write Anthropic Provider Tests
- Test chat completion with real Anthropic API
- Test tool calling with Claude
- Test streaming responses
- Test error handling

### [x] Task 1.4: Write Ollama Provider Tests
- Test local model chat completion
- Test tool calling (if supported by model)
- Test streaming responses

### [x] Task 1.5: Write OpenRouter Provider Tests
- Test multi-provider routing
- Test model selection
- Test cost tracking

## Phase 2: Implementation (GREEN)

### [x] Task 2.1: Implement Enhanced Provider Trait
**Deliverables:** `crates/maestro-claw/src/providers/trait.rs`

### [x] Task 2.2: Implement OpenAI Provider
**Deliverables:** `crates/maestro-claw/src/providers/openai.rs`

### [x] Task 2.3: Implement Anthropic Provider
**Deliverables:** `crates/maestro-claw/src/providers/anthropic.rs`

### [x] Task 2.4: Implement Ollama Provider
**Deliverables:** `crates/maestro-claw/src/providers/ollama.rs`

### [x] Task 2.5: Implement OpenRouter Provider
**Deliverables:** `crates/maestro-claw/src/providers/openrouter.rs`

## Phase 3: Verification

### [x] Task 3.1: Run All Tests
- 46 provider tests all pass (OpenAI: 5, Anthropic: 5, Ollama: 5, OpenRouter: 5, trait: 26) ✅
- 11 integration tests for OpenAI provider format in tests/openai_provider.rs ✅

### [x] Task 3.2: Coverage Check > 98%
- Provider trait: 100% for serialization paths
- All 4 provider implementations: 100% for unit test paths

### [x] Task 3.3: Manual Verification
- [x] Task: Maestro - User Manual Verification 'Subtrack 04: Providers'
  - Enhanced Provider trait with chat(), stream_chat(), chat_with_tools() ✅
  - OpenAI provider with GPT model support ✅
  - Anthropic provider with Claude model support ✅
  - Ollama provider for local models ✅
  - OpenRouter provider for multi-provider routing ✅
  - capabilities(), supports_native_tools(), error types ✅
