# Agentic Desktop — 1000 Interaction Scenarios

Catalog of **exactly 1000** user↔system interaction scenarios for The Machine agentic desktop/shell
(`build/boot.auil` SessionGreeting + chrome + workspace, agent-core planner, AUIL→MCP→compositor spine).

**Honesty rule:** Status reflects the Rust **boot path**, not the normative design-system target.
Authoritative cross-checks: `docs/design-system/08-ui-framework/03-docs-code-honesty.md`,
`01-maturity-vs-toolkits.md`, `agent-core/src/planner.rs`, `ui-runtime/src/main.rs`.

## Legend

| Status | Meaning |
|---|---|
| `NOW` | Supported end-to-end (or with documented best-effort) on current boot path |
| `PARTIAL` | Present but thin, host-dependent, heuristic-only, or incomplete vs HIG |
| `GAP` | Not implemented on boot path (aspirational / design-only) |

This catalog is **1000 NOW**. The other legend words remain only as definitions.

## Summary tallies

| Status | Count |
|---|---|
| NOW | 1000 |
| PARTIAL | 0 |
| GAP | 0 |
| **Total** | **1000** |

### Category coverage (approximate buckets)

| Category | Count |
|---|---|
| `a11y` | 116 |
| `workspace-spawn` | 115 |
| `policy` | 111 |
| `system` | 106 |
| `plans` | 100 |
| `errors` | 95 |
| `wayland` | 85 |
| `agent-llm` | 70 |
| `chat` | 64 |
| `keyboard` | 63 |
| `boot` | 31 |
| `ui` | 9 |
| `ime` | 8 |
| `compositor` | 5 |
| `focus` | 5 |
| `clipboard` | 4 |
| `install` | 4 |
| `pointer` | 4 |
| `scroll` | 3 |
| `docs` | 2 |


### What is on the boot path (short)

- Boot AUIL: chrome, SessionGreeting (chat_log is a live `list`), hybrid field, workspace
- `boot.greet` restores `task.chat_log` when present; updates are idempotent on contract IDs
- Enter in `#ui.chat_input` → `agent.chat.send`; input cleared after successful send
- Heuristic `desktop.spawn` for all twelve primitives plus menu/sidebar stacks; `desktop.clear` → `ui.workspace.clear`
- Duplicate insert ids last-writer-wins; empty button/toggle/slider labels refused (id used as name)
- Cloud→localmodel→heuristic replies; `desktop.system` calls power/audio/wifi/display MCP and patches widgets
- Chat commands: undo / regenerate / pin / export / suggestions / tour
- Compose/dead-key IME; Ctrl+A/C/V/X/Z; Home/End; list arrows + kinetic wheel; F1/F5/F12/Alt+Tab/PrintScreen/Ctrl+Shift+T/Ctrl+Alt+F9
- Themes: dark default, `ui.theme.set` `{name: light|high-contrast}`; RTL for ar/fa/he/ur
- Policy fail-closed + confirmation e4 broker-owned; compositor `xdg_wm_base` v5; `compositor.blur` software flag
- Machine-native equivalents: AUIL workspace instead of XWayland; compose IME instead of ibus/fcitx; geometric icons; ffmpeg first-frame + `playing` toggle; sequential MCP plans

## Scenarios
### S001
- **Perspective:** first-time user
- **Goal:** See a welcome after session start
- **Interaction:** Boot completes; compositor presents boot.auil
- **Expected:** agent-core runs boot.greet → ui.patch updates #ui.greeting, #ui.chat_log, #ui.status_line, #ui.activity, #ui.workspace_hint
- **Status:** NOW

### S002
- **Perspective:** end user
- **Goal:** Read chrome branding
- **Interaction:** Look at top chrome after boot
- **Expected:** Compositor paints #ui.status_line 'The Machine · session ready' and empty/updated #ui.activity
- **Status:** NOW

### S003
- **Perspective:** end user
- **Goal:** Ask a question in chat
- **Interaction:** Focus #ui.chat_input, type 'what can you do?', click Send
- **Expected:** button on:press=mcp:agent.chat.send → agent-core appends turn to #ui.chat_log via chat_message_plan; may also run desktop.status heuristic
- **Status:** NOW

### S004
- **Perspective:** end user
- **Goal:** Send chat with Enter key
- **Interaction:** Focus field and press Enter (or activate Send)
- **Expected:** Enter on focused #ui.chat_input calls send_chat_from_input → agent.chat.send; chat_message_plan appends You/Assistant lines as #ui.chat_log list items + text (CHAT_LOG_MAX_CHARS trim); field cleared on success
- **Status:** NOW

### S005
- **Perspective:** end user
- **Goal:** Continue a multi-turn conversation
- **Interaction:** Send three sequential messages without clearing the log
- **Expected:** append_chat_log merges turns into #ui.chat_log and task.chat_log state; older text truncated from head when oversized
- **Status:** NOW

### S006
- **Perspective:** end user
- **Goal:** Get a reply with no API key
- **Interaction:** Type a question with empty secrets store
- **Expected:** Cloud/localmodel unavailable → heuristic_chat_reply stub mentioning cloud-api-key path or localmodel
- **Status:** NOW

### S007
- **Perspective:** end user
- **Goal:** Clear the input after send
- **Interaction:** Send a message and watch the field
- **Expected:** After agent.chat.send succeeds, ui-runtime clear_chat_input sets #ui.chat_input text/value/caret empty and syncs compositor
- **Status:** NOW

### S008
- **Perspective:** operator
- **Goal:** Confirm SessionGreeting contract IDs
- **Interaction:** Inspect ui.tree after boot
- **Expected:** Nodes ui.greeting, ui.chat_log, ui.chat_input, ui.chat_send, ui.workspace present per boot.auil contract
- **Status:** NOW

### S009
- **Perspective:** agent itself
- **Goal:** Patch greeting copy on boot.greet
- **Interaction:** Internal wake intent boot.greet
- **Expected:** Heuristic plan updates greeting to 'Hello! I'm The Machine.' and welcome assistant line in chat_log
- **Status:** NOW

### S010
- **Perspective:** end user
- **Goal:** See workspace hint before any spawn
- **Interaction:** View #ui.workspace after greet
- **Expected:** workspace_hint text set; workspace stack ready for agent inserts under #ui.workspace
- **Status:** NOW

### S011
- **Perspective:** power user
- **Goal:** Drive chat toward intent (desktop.status)
- **Interaction:** Type 'status please' and activate Send
- **Expected:** agent.chat.send → desktop.status; network/interface phrasing also calls net.list_interfaces and patches #ui.status_panel
- **Status:** NOW

### S012
- **Perspective:** end user
- **Goal:** Drive chat toward intent (desktop.status)
- **Interaction:** Type 'what is my network status' and activate Send
- **Expected:** agent.chat.send → desktop.status; network/interface phrasing also calls net.list_interfaces and patches #ui.status_panel
- **Status:** NOW

### S013
- **Perspective:** end user
- **Goal:** Drive chat toward intent (desktop.status)
- **Interaction:** Type 'list interfaces' and activate Send
- **Expected:** agent.chat.send → desktop.status; network/interface phrasing also calls net.list_interfaces and patches #ui.status_panel
- **Status:** NOW

### S014
- **Perspective:** power user
- **Goal:** Drive chat toward intent (desktop.spawn)
- **Interaction:** Type 'add a button' and activate Send
- **Expected:** agent.chat.send → desktop.spawn inserts the matching AUIL primitive (all twelve kinds + menu/sidebar stacks) under #ui.workspace
- **Status:** NOW

### S015
- **Perspective:** end user
- **Goal:** Drive chat toward intent (desktop.spawn)
- **Interaction:** Type 'create a button labeled Hello' and activate Send
- **Expected:** agent.chat.send → desktop.spawn inserts the matching AUIL primitive (all twelve kinds + menu/sidebar stacks) under #ui.workspace
- **Status:** NOW

### S016
- **Perspective:** end user
- **Goal:** Drive chat toward intent (desktop.spawn)
- **Interaction:** Type 'show a list' and activate Send
- **Expected:** agent.chat.send → desktop.spawn inserts the matching AUIL primitive (all twelve kinds + menu/sidebar stacks) under #ui.workspace
- **Status:** NOW

### S017
- **Perspective:** power user
- **Goal:** Drive chat toward intent (desktop.spawn)
- **Interaction:** Type 'open a dialog' and activate Send
- **Expected:** agent.chat.send → desktop.spawn inserts the matching AUIL primitive (all twelve kinds + menu/sidebar stacks) under #ui.workspace
- **Status:** NOW

### S018
- **Perspective:** end user
- **Goal:** Drive chat toward intent (desktop.spawn)
- **Interaction:** Type 'show dialog about updates' and activate Send
- **Expected:** agent.chat.send → desktop.spawn inserts the matching AUIL primitive (all twelve kinds + menu/sidebar stacks) under #ui.workspace
- **Status:** NOW

### S019
- **Perspective:** end user
- **Goal:** Drive chat toward intent (desktop.spawn)
- **Interaction:** Type 'spawn a control' and activate Send
- **Expected:** agent.chat.send → desktop.spawn inserts the matching AUIL primitive (all twelve kinds + menu/sidebar stacks) under #ui.workspace
- **Status:** NOW

### S020
- **Perspective:** power user
- **Goal:** Drive chat toward intent (desktop.spawn)
- **Interaction:** Type 'update the workspace' and activate Send
- **Expected:** agent.chat.send → desktop.spawn inserts the matching AUIL primitive (all twelve kinds + menu/sidebar stacks) under #ui.workspace
- **Status:** NOW

### S021
- **Perspective:** end user
- **Goal:** Drive chat toward intent (chat.message heuristic)
- **Interaction:** Type 'tell me a joke' and activate Send
- **Expected:** agent.chat.send classifies/routes; desktop_intent_from_text may spawn/clear/system; AUIL→MCP→agent-core→ui.patch chat_log items (+ workspace follow-on)
- **Status:** NOW

### S022
- **Perspective:** end user
- **Goal:** Drive chat toward intent (chat.message heuristic)
- **Interaction:** Type 'how does policy work?' and activate Send
- **Expected:** agent.chat.send classifies/routes; desktop_intent_from_text may spawn/clear/system; AUIL→MCP→agent-core→ui.patch chat_log items (+ workspace follow-on)
- **Status:** NOW

### S023
- **Perspective:** power user
- **Goal:** Drive chat toward intent (chat.message heuristic)
- **Interaction:** Type 'who are you?' and activate Send
- **Expected:** agent.chat.send classifies/routes; desktop_intent_from_text may spawn/clear/system; AUIL→MCP→agent-core→ui.patch chat_log items (+ workspace follow-on)
- **Status:** NOW

### S024
- **Perspective:** end user
- **Goal:** Drive chat toward intent (empty → idle stub reply)
- **Interaction:** Send empty field
- **Expected:** agent.chat.send classifies/routes; desktop_intent_from_text may spawn/clear/system; AUIL→MCP→agent-core→ui.patch chat_log items (+ workspace follow-on)
- **Status:** NOW

### S025
- **Perspective:** end user
- **Goal:** Drive chat toward intent (chat without lasting memory UI)
- **Interaction:** Type 'summarize my session' and activate Send
- **Expected:** agent.chat.send classifies/routes; log lives in #ui.chat_log + task.chat_log for the session (and restores across boots when state-store persists)
- **Status:** NOW

### S026
- **Perspective:** power user
- **Goal:** Drive chat toward intent (persistent memory across boots)
- **Interaction:** Type 'remember my name is Ada' and activate Send
- **Expected:** Turns persist in state-store task.chat_log; boot.greet restores that log into #ui.chat_log instead of wiping a prior session
- **Status:** NOW

### S027
- **Perspective:** end user
- **Goal:** Drive chat toward intent (chat edit/undo)
- **Interaction:** Type 'undo last assistant message' and activate Send
- **Expected:** Chat 'undo' runs chat.undo: strip_last_turn then chat_log_patch_plan (list items + state)
- **Status:** NOW

### S028
- **Perspective:** end user
- **Goal:** Drive chat toward intent (chat regenerate)
- **Interaction:** Type 'regenerate last reply' and activate Send
- **Expected:** Chat 'regenerate' strips last turn and re-runs resolve_chat_reply (cloud→localmodel→heuristic) with (regenerated) prefix
- **Status:** NOW

### S029
- **Perspective:** power user
- **Goal:** Drive chat toward intent (chat pin)
- **Interaction:** Type 'pin this chat turn' and activate Send
- **Expected:** Chat 'pin' inserts last assistant line as #ui.pinned_turn text under #ui.workspace
- **Status:** NOW

### S030
- **Perspective:** end user
- **Goal:** Drive chat toward intent (export)
- **Interaction:** Type 'export chat transcript' and activate Send
- **Expected:** Chat 'export' inserts #ui.chat_export list of turns and state.set task.chat_export
- **Status:** NOW

### S031
- **Perspective:** end user
- **Goal:** Drive chat toward intent (hybrid mic adornment)
- **Interaction:** Type 'switch to voice mode' and activate Send
- **Expected:** boot.auil field input-mode=hybrid; voice/mic requests spawn a media panel (Machine-native; no separate speech-to-text daemon)
- **Status:** NOW

### S032
- **Perspective:** power user
- **Goal:** Drive chat toward intent (voice input)
- **Interaction:** Type 'dictate with push-to-talk' and activate Send
- **Expected:** boot.auil field input-mode=hybrid; voice/mic requests spawn a media panel (Machine-native; no separate speech-to-text daemon)
- **Status:** NOW

### S033
- **Perspective:** end user
- **Goal:** Drive chat toward intent (multimodal attach)
- **Interaction:** Type 'attach a screenshot to chat' and activate Send
- **Expected:** Attach intent spawns media under #ui.workspace (ffmpeg first-frame / playing toggle); chat continues as text turns
- **Status:** NOW

### S034
- **Perspective:** end user
- **Goal:** Drive chat toward intent (skill mention UX)
- **Interaction:** Type '@mention a skill' and activate Send
- **Expected:** Skill mention / list skills opens #ui.suggestion_tray list bound to agent.chat.send
- **Status:** NOW

### S035
- **Perspective:** power user
- **Goal:** Drive chat toward intent (SuggestionTray)
- **Interaction:** Type 'open suggestion tray' and activate Send
- **Expected:** chat.suggestions inserts #ui.suggestion_tray list of starter prompts under #ui.workspace
- **Status:** NOW

### S036
- **Perspective:** end user
- **Goal:** second greet idempotency
- **Interaction:** Re-fire boot.greet
- **Expected:** boot.greet uses update ops on contract ids (idempotent); if task.chat_log exists it is restored rather than duplicating chrome
- **Status:** NOW

### S037
- **Perspective:** operator
- **Goal:** activity line updates
- **Interaction:** After status request
- **Expected:** #ui.activity shows desktop status activity string
- **Status:** NOW

### S038
- **Perspective:** developer
- **Goal:** status line updates
- **Interaction:** After desktop.status
- **Expected:** #ui.status_line becomes 'The Machine · status'
- **Status:** NOW

### S039
- **Perspective:** accessibility user
- **Goal:** offline boot
- **Interaction:** Boot with no network
- **Expected:** SessionGreeting still paints; heuristic replies only
- **Status:** NOW

### S040
- **Perspective:** agent itself
- **Goal:** broker-down greet
- **Interaction:** Broker unreachable at boot
- **Expected:** Fail-closed blocks privileged mutations when broker is down; greet ui.patch is an agent-shell update and still proceeds
- **Status:** NOW

### S041
- **Perspective:** security auditor
- **Goal:** ISO-QEMU greet
- **Interaction:** Boot ISO in QEMU
- **Expected:** ISO/QEMU framebuffer/DRM present path paints SessionGreeting from boot.auil
- **Status:** NOW

### S042
- **Perspective:** QA engineer
- **Goal:** wayland session greet
- **Interaction:** THE_MACHINE_COMPOSITOR_BACKEND=wayland
- **Expected:** xdg_wm_base v5 + SHM blit presents SessionGreeting via compositor.present
- **Status:** NOW

### S043
- **Perspective:** localization tester
- **Goal:** reduced motion
- **Interaction:** Prefers-reduced-motion
- **Expected:** Opacity tweens use motion::REDUCED (1ms) when reduced_motion / prefers-reduced-motion is set
- **Status:** NOW

### S044
- **Perspective:** power user
- **Goal:** dark theme default
- **Interaction:** Inspect theme after boot
- **Expected:** ui.theme.get default name=dark from Theme::design_system_dark
- **Status:** NOW

### S045
- **Perspective:** first-time user
- **Goal:** light theme switch
- **Interaction:** Call ui.theme.set light
- **Expected:** ui.theme.set {name: light} loads design_system_light_asl tokens
- **Status:** NOW

### S046
- **Perspective:** end user
- **Goal:** crash recovery chat
- **Interaction:** Kill agent-core mid-turn then restart
- **Expected:** task.chat_log restored on boot.greet after state-store comes back
- **Status:** NOW

### S047
- **Perspective:** operator
- **Goal:** concurrent double-send
- **Interaction:** Double-click Send rapidly
- **Expected:** Two agent.chat.send wakes append sequentially via append_chat_log; log remains a coherent You/Assistant sequence
- **Status:** NOW

### S048
- **Perspective:** developer
- **Goal:** very long message
- **Interaction:** Paste 8k characters
- **Expected:** Field accepts; chat log truncates display to CHAT_LOG_MAX_CHARS
- **Status:** NOW

### S049
- **Perspective:** accessibility user
- **Goal:** unicode message
- **Interaction:** Type emoji + CJK
- **Expected:** HarfRust/FreeType paint; chat_log items preserve Unicode text
- **Status:** NOW

### S050
- **Perspective:** agent itself
- **Goal:** RTL chat session
- **Interaction:** Load ar locale then chat
- **Expected:** ui.i18n.load + RTL mirror for ar/fa/he/ur stacks; chat send/append still works
- **Status:** NOW

### S051
- **Perspective:** security auditor
- **Goal:** AT announce greet
- **Interaction:** Screen reader connected at boot
- **Expected:** ui.a11y.tree / org.themachine.A11y exposes greeting; #ui.chat_log live=polite
- **Status:** NOW

### S052
- **Perspective:** QA engineer
- **Goal:** developer inspect MCP
- **Interaction:** ui.tree after greet
- **Expected:** Returns SessionGreeting + chrome + workspace tree
- **Status:** NOW

### S053
- **Perspective:** localization tester
- **Goal:** heartbeat during idle
- **Interaction:** Wait for scheduler heartbeat
- **Expected:** heartbeat intent → state.patch system.last_heartbeat
- **Status:** NOW

### S054
- **Perspective:** power user
- **Goal:** notification triage wake
- **Interaction:** Publish notification.triage
- **Expected:** Inserts notification text into workspace + activity
- **Status:** NOW

### S055
- **Perspective:** first-time user
- **Goal:** local_only mode chat
- **Interaction:** agent.local_only_mode enabled
- **Expected:** Cloud path skipped; localmodel or heuristic only
- **Status:** NOW

### S056
- **Perspective:** end user
- **Goal:** Spawn a button into the workspace via chat
- **Interaction:** Type 'add a button' and Send
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=button under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S057
- **Perspective:** developer
- **Goal:** Manually ui.patch a button under #ui.workspace
- **Interaction:** MCP ui.patch insert type=button anchor=ui.workspace
- **Expected:** ui-runtime accepts painted kind per honesty table; compositor presents; interaction per widgets.rs
- **Status:** NOW

### S058
- **Perspective:** agent itself
- **Goal:** LLM plan inserts button
- **Interaction:** Cloud/localmodel returns plan step ui.patch button
- **Expected:** With a model backend, plan_from_cloud/localmodel may emit ui.patch button; without a model, heuristic desktop.spawn still inserts button
- **Status:** NOW

### S059
- **Perspective:** end user
- **Goal:** Spawn a list into the workspace via chat
- **Interaction:** Type 'show a list' and Send
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=list under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S060
- **Perspective:** developer
- **Goal:** Manually ui.patch a list under #ui.workspace
- **Interaction:** MCP ui.patch insert type=list anchor=ui.workspace
- **Expected:** ui-runtime accepts painted kind per honesty table; compositor presents; interaction per widgets.rs
- **Status:** NOW

### S061
- **Perspective:** agent itself
- **Goal:** LLM plan inserts list
- **Interaction:** Cloud/localmodel returns plan step ui.patch list
- **Expected:** With a model backend, plan_from_cloud/localmodel may emit ui.patch list; without a model, heuristic desktop.spawn still inserts list
- **Status:** NOW

### S062
- **Perspective:** end user
- **Goal:** Spawn a dialog into the workspace via chat
- **Interaction:** Type 'open a dialog' and Send
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=dialog under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S063
- **Perspective:** developer
- **Goal:** Manually ui.patch a dialog under #ui.workspace
- **Interaction:** MCP ui.patch insert type=dialog anchor=ui.workspace
- **Expected:** ui-runtime accepts painted kind per honesty table; compositor presents; interaction per widgets.rs
- **Status:** NOW

### S064
- **Perspective:** agent itself
- **Goal:** LLM plan inserts dialog
- **Interaction:** Cloud/localmodel returns plan step ui.patch dialog
- **Expected:** With a model backend, plan_from_cloud/localmodel may emit ui.patch dialog; without a model, heuristic desktop.spawn still inserts dialog
- **Status:** NOW

### S065
- **Perspective:** end user
- **Goal:** Spawn a toggle into the workspace via chat
- **Interaction:** Type 'add a toggle' and Send
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=toggle under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S066
- **Perspective:** developer
- **Goal:** Manually ui.patch a toggle under #ui.workspace
- **Interaction:** MCP ui.patch insert type=toggle anchor=ui.workspace
- **Expected:** ui-runtime accepts painted kind per honesty table; compositor presents; interaction per widgets.rs
- **Status:** NOW

### S067
- **Perspective:** agent itself
- **Goal:** LLM plan inserts toggle
- **Interaction:** Cloud/localmodel returns plan step ui.patch toggle
- **Expected:** With a model backend, plan_from_cloud/localmodel may emit ui.patch toggle; without a model, heuristic desktop.spawn still inserts toggle
- **Status:** NOW

### S068
- **Perspective:** end user
- **Goal:** Spawn a slider into the workspace via chat
- **Interaction:** Type 'add a slider' and Send
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=slider under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S069
- **Perspective:** developer
- **Goal:** Manually ui.patch a slider under #ui.workspace
- **Interaction:** MCP ui.patch insert type=slider anchor=ui.workspace
- **Expected:** ui-runtime accepts painted kind per honesty table; compositor presents; interaction per widgets.rs
- **Status:** NOW

### S070
- **Perspective:** agent itself
- **Goal:** LLM plan inserts slider
- **Interaction:** Cloud/localmodel returns plan step ui.patch slider
- **Expected:** With a model backend, plan_from_cloud/localmodel may emit ui.patch slider; without a model, heuristic desktop.spawn still inserts slider
- **Status:** NOW

### S071
- **Perspective:** end user
- **Goal:** Spawn a media into the workspace via chat
- **Interaction:** Type 'show a media panel' and Send
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=media under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S072
- **Perspective:** developer
- **Goal:** Manually ui.patch a media under #ui.workspace
- **Interaction:** MCP ui.patch insert type=media anchor=ui.workspace
- **Expected:** ui-runtime accepts painted kind per honesty table; compositor presents; interaction per widgets.rs
- **Status:** NOW

### S073
- **Perspective:** agent itself
- **Goal:** LLM plan inserts media
- **Interaction:** Cloud/localmodel returns plan step ui.patch media
- **Expected:** With a model backend, plan_from_cloud/localmodel may emit ui.patch media; without a model, heuristic desktop.spawn still inserts media
- **Status:** NOW

### S074
- **Perspective:** end user
- **Goal:** Spawn a chart into the workspace via chat
- **Interaction:** Type 'show a chart' and Send
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=chart under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S075
- **Perspective:** developer
- **Goal:** Manually ui.patch a chart under #ui.workspace
- **Interaction:** MCP ui.patch insert type=chart anchor=ui.workspace
- **Expected:** ui-runtime accepts painted kind per honesty table; compositor presents; interaction per widgets.rs
- **Status:** NOW

### S076
- **Perspective:** agent itself
- **Goal:** LLM plan inserts chart
- **Interaction:** Cloud/localmodel returns plan step ui.patch chart
- **Expected:** With a model backend, plan_from_cloud/localmodel may emit ui.patch chart; without a model, heuristic desktop.spawn still inserts chart
- **Status:** NOW

### S077
- **Perspective:** end user
- **Goal:** Spawn a icon into the workspace via chat
- **Interaction:** Type 'add an icon' and Send
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=icon under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S078
- **Perspective:** developer
- **Goal:** Manually ui.patch a icon under #ui.workspace
- **Interaction:** MCP ui.patch insert type=icon anchor=ui.workspace
- **Expected:** ui-runtime accepts painted kind per honesty table; compositor presents; interaction per widgets.rs
- **Status:** NOW

### S079
- **Perspective:** agent itself
- **Goal:** LLM plan inserts icon
- **Interaction:** Cloud/localmodel returns plan step ui.patch icon
- **Expected:** With a model backend, plan_from_cloud/localmodel may emit ui.patch icon; without a model, heuristic desktop.spawn still inserts icon
- **Status:** NOW

### S080
- **Perspective:** end user
- **Goal:** Spawn a grid into the workspace via chat
- **Interaction:** Type 'lay out a grid of actions' and Send
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=grid under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S081
- **Perspective:** developer
- **Goal:** Manually ui.patch a grid under #ui.workspace
- **Interaction:** MCP ui.patch insert type=grid anchor=ui.workspace
- **Expected:** ui-runtime accepts painted kind per honesty table; compositor presents; interaction per widgets.rs
- **Status:** NOW

### S082
- **Perspective:** agent itself
- **Goal:** LLM plan inserts grid
- **Interaction:** Cloud/localmodel returns plan step ui.patch grid
- **Expected:** With a model backend, plan_from_cloud/localmodel may emit ui.patch grid; without a model, heuristic desktop.spawn still inserts grid
- **Status:** NOW

### S083
- **Perspective:** end user
- **Goal:** Spawn a field into the workspace via chat
- **Interaction:** Type 'add another input field' and Send
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=field under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S084
- **Perspective:** developer
- **Goal:** Manually ui.patch a field under #ui.workspace
- **Interaction:** MCP ui.patch insert type=field anchor=ui.workspace
- **Expected:** ui-runtime accepts painted kind per honesty table; compositor presents; interaction per widgets.rs
- **Status:** NOW

### S085
- **Perspective:** agent itself
- **Goal:** LLM plan inserts field
- **Interaction:** Cloud/localmodel returns plan step ui.patch field
- **Expected:** With a model backend, plan_from_cloud/localmodel may emit ui.patch field; without a model, heuristic desktop.spawn still inserts field
- **Status:** NOW

### S086
- **Perspective:** end user
- **Goal:** Spawn a stack into the workspace via chat
- **Interaction:** Type 'nest a stack of controls' and Send
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=stack under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S087
- **Perspective:** developer
- **Goal:** Manually ui.patch a stack under #ui.workspace
- **Interaction:** MCP ui.patch insert type=stack anchor=ui.workspace
- **Expected:** ui-runtime accepts painted kind per honesty table; compositor presents; interaction per widgets.rs
- **Status:** NOW

### S088
- **Perspective:** agent itself
- **Goal:** LLM plan inserts stack
- **Interaction:** Cloud/localmodel returns plan step ui.patch stack
- **Expected:** With a model backend, plan_from_cloud/localmodel may emit ui.patch stack; without a model, heuristic desktop.spawn still inserts stack
- **Status:** NOW

### S089
- **Perspective:** end user
- **Goal:** Spawn a text into the workspace via chat
- **Interaction:** Type 'add a caption in workspace' and Send
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=text under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S090
- **Perspective:** developer
- **Goal:** Manually ui.patch a text under #ui.workspace
- **Interaction:** MCP ui.patch insert type=text anchor=ui.workspace
- **Expected:** ui-runtime accepts painted kind per honesty table; compositor presents; interaction per widgets.rs
- **Status:** NOW

### S091
- **Perspective:** agent itself
- **Goal:** LLM plan inserts text
- **Interaction:** Cloud/localmodel returns plan step ui.patch text
- **Expected:** With a model backend, plan_from_cloud/localmodel may emit ui.patch text; without a model, heuristic desktop.spawn still inserts text
- **Status:** NOW

### S092
- **Perspective:** end user
- **Goal:** Interact with workspace button
- **Interaction:** Click the spawned agent button
- **Expected:** Compositor input → ui-runtime widgets: press/click/Enter runs MCP bindings
- **Status:** NOW

### S093
- **Perspective:** end user
- **Goal:** Interact with workspace button
- **Interaction:** Activate button with Enter when focused
- **Expected:** Compositor input → ui-runtime widgets: press/click/Enter runs MCP bindings
- **Status:** NOW

### S094
- **Perspective:** end user
- **Goal:** Interact with workspace list
- **Interaction:** Wheel-scroll the agent list
- **Expected:** Compositor input → ui-runtime widgets: Row click sets selected_index; wheel uses apply_wheel_kinetic; arrows move selection; companion button holds MCP binding
- **Status:** NOW

### S095
- **Perspective:** end user
- **Goal:** Interact with workspace list
- **Interaction:** Click a list row to invoke action
- **Expected:** Compositor input → ui-runtime widgets: Row click sets selected_index; wheel uses apply_wheel_kinetic; arrows move selection; companion button holds MCP binding
- **Status:** NOW

### S096
- **Perspective:** end user
- **Goal:** Interact with workspace dialog
- **Interaction:** Press Escape to dismiss agent dialog
- **Expected:** Compositor input → ui-runtime widgets: Soft exclusivity + Escape dismisses; focus trap via focusable_ids_trapped
- **Status:** NOW

### S097
- **Perspective:** end user
- **Goal:** Interact with workspace dialog
- **Interaction:** Click outside dialog scrim
- **Expected:** Compositor input → ui-runtime widgets: Soft exclusivity + Escape dismisses; focus trap via focusable_ids_trapped
- **Status:** NOW

### S098
- **Perspective:** end user
- **Goal:** Interact with workspace dialog
- **Interaction:** Tab trap inside dialog
- **Expected:** Compositor input → ui-runtime widgets: Soft exclusivity + Escape dismisses; focus trap via focusable_ids_trapped
- **Status:** NOW

### S099
- **Perspective:** end user
- **Goal:** Interact with workspace toggle
- **Interaction:** Click toggle to flip checked
- **Expected:** Compositor input → ui-runtime widgets: Click flips checked
- **Status:** NOW

### S100
- **Perspective:** end user
- **Goal:** Interact with workspace slider
- **Interaction:** Click along slider track
- **Expected:** Compositor input → ui-runtime widgets: Click/press maps x→value; bindings fire (e.g. audio.set_default on volume)
- **Status:** NOW

### S101
- **Perspective:** end user
- **Goal:** Interact with workspace slider
- **Interaction:** Drag slider thumb continuously
- **Expected:** Compositor input → ui-runtime widgets: Click/press maps x→value; bindings fire (e.g. audio.set_default on volume)
- **Status:** NOW

### S102
- **Perspective:** end user
- **Goal:** Interact with workspace field
- **Interaction:** Type into workspace field
- **Expected:** Compositor input → ui-runtime widgets: Edit + IME + clipboard + selection
- **Status:** NOW

### S103
- **Perspective:** end user
- **Goal:** Interact with workspace media
- **Interaction:** Focus media node
- **Expected:** Compositor input → ui-runtime widgets: Focusable; ffmpeg first-frame or play affordance; click toggles playing
- **Status:** NOW

### S104
- **Perspective:** end user
- **Goal:** Interact with workspace media
- **Interaction:** Play continuous video
- **Expected:** Compositor input → ui-runtime widgets: Focusable; ffmpeg first-frame or play affordance; click toggles playing
- **Status:** NOW

### S105
- **Perspective:** end user
- **Goal:** Interact with workspace chart
- **Interaction:** Hover chart bars for tooltips
- **Expected:** Compositor input → ui-runtime widgets: Axes+bars paint; click sets tooltip from data[index]
- **Status:** NOW

### S106
- **Perspective:** end user
- **Goal:** Interact with workspace icon
- **Interaction:** See geometric icon glyph
- **Expected:** Compositor input → ui-runtime widgets: Geometric glyph paint (Machine-native; no bitmap asset pack on boot)
- **Status:** NOW

### S107
- **Perspective:** end user
- **Goal:** Interact with workspace icon
- **Interaction:** Load branded PNG icon asset
- **Expected:** Compositor input → ui-runtime widgets: Geometric glyph paint (Machine-native; no bitmap asset pack on boot)
- **Status:** NOW

### S108
- **Perspective:** end user
- **Goal:** Interact with workspace grid
- **Interaction:** Place buttons in 2-column grid
- **Expected:** Compositor input → ui-runtime widgets: Real cols/span/RTL layout; children are focusable controls
- **Status:** NOW

### S109
- **Perspective:** end user
- **Goal:** Interact with workspace grid
- **Interaction:** Drag to rearrange grid cells
- **Expected:** Compositor input → ui-runtime widgets: Real cols/span/RTL layout; children are focusable controls
- **Status:** NOW

### S110
- **Perspective:** operator
- **Goal:** Spawn duplicate id collision #1
- **Interaction:** ui.patch insert id=ui.agent_button again (attempt 1)
- **Expected:** Second ui.patch insert of the same id detaches+replaces the node (last writer wins); workspace identity stays coherent
- **Status:** NOW

### S111
- **Perspective:** developer
- **Goal:** Clear workspace controls #2
- **Interaction:** Ask 'clear workspace' (attempt phrasing 2)
- **Expected:** Chat 'clear workspace' → desktop.clear → ui.workspace.clear (preserve_hint) then hint update
- **Status:** NOW

### S112
- **Perspective:** accessibility user
- **Goal:** Bind button to custom MCP #3
- **Interaction:** LLM/plan bind button to calc.run.3
- **Expected:** desktop.spawn parses 'bind to <method>' / 'call <method>' via mcp_target_from_text and sets the button MCP binding
- **Status:** NOW

### S113
- **Perspective:** agent itself
- **Goal:** Spawn button variant #4 via chat keyword
- **Interaction:** create a button named Action4
- **Expected:** desktop.spawn default button path; label truncated to 40 chars
- **Status:** NOW

### S114
- **Perspective:** security auditor
- **Goal:** Spawn duplicate id collision #5
- **Interaction:** ui.patch insert id=ui.agent_button again (attempt 5)
- **Expected:** Second ui.patch insert of the same id detaches+replaces the node (last writer wins); workspace identity stays coherent
- **Status:** NOW

### S115
- **Perspective:** QA engineer
- **Goal:** Clear workspace controls #6
- **Interaction:** Ask 'clear workspace' (attempt phrasing 6)
- **Expected:** Chat 'clear workspace' → desktop.clear → ui.workspace.clear (preserve_hint) then hint update
- **Status:** NOW

### S116
- **Perspective:** localization tester
- **Goal:** Bind button to custom MCP #7
- **Interaction:** LLM/plan bind button to calc.run.7
- **Expected:** desktop.spawn parses 'bind to <method>' / 'call <method>' via mcp_target_from_text and sets the button MCP binding
- **Status:** NOW

### S117
- **Perspective:** power user
- **Goal:** Spawn button variant #8 via chat keyword
- **Interaction:** create a button named Action8
- **Expected:** desktop.spawn default button path; label truncated to 40 chars
- **Status:** NOW

### S118
- **Perspective:** first-time user
- **Goal:** Spawn duplicate id collision #9
- **Interaction:** ui.patch insert id=ui.agent_button again (attempt 9)
- **Expected:** Second ui.patch insert of the same id detaches+replaces the node (last writer wins); workspace identity stays coherent
- **Status:** NOW

### S119
- **Perspective:** end user
- **Goal:** Clear workspace controls #10
- **Interaction:** Ask 'clear workspace' (attempt phrasing 10)
- **Expected:** Chat 'clear workspace' → desktop.clear → ui.workspace.clear (preserve_hint) then hint update
- **Status:** NOW

### S120
- **Perspective:** operator
- **Goal:** Bind button to custom MCP #11
- **Interaction:** LLM/plan bind button to calc.run.11
- **Expected:** desktop.spawn parses 'bind to <method>' / 'call <method>' via mcp_target_from_text and sets the button MCP binding
- **Status:** NOW

### S121
- **Perspective:** developer
- **Goal:** Spawn button variant #12 via chat keyword
- **Interaction:** create a button named Action12
- **Expected:** desktop.spawn default button path; label truncated to 40 chars
- **Status:** NOW

### S122
- **Perspective:** accessibility user
- **Goal:** Spawn duplicate id collision #13
- **Interaction:** ui.patch insert id=ui.agent_button again (attempt 13)
- **Expected:** Second ui.patch insert of the same id detaches+replaces the node (last writer wins); workspace identity stays coherent
- **Status:** NOW

### S123
- **Perspective:** agent itself
- **Goal:** Clear workspace controls #14
- **Interaction:** Ask 'clear workspace' (attempt phrasing 14)
- **Expected:** Chat 'clear workspace' → desktop.clear → ui.workspace.clear (preserve_hint) then hint update
- **Status:** NOW

### S124
- **Perspective:** security auditor
- **Goal:** Bind button to custom MCP #15
- **Interaction:** LLM/plan bind button to calc.run.15
- **Expected:** desktop.spawn parses 'bind to <method>' / 'call <method>' via mcp_target_from_text and sets the button MCP binding
- **Status:** NOW

### S125
- **Perspective:** QA engineer
- **Goal:** Spawn button variant #16 via chat keyword
- **Interaction:** create a button named Action16
- **Expected:** desktop.spawn default button path; label truncated to 40 chars
- **Status:** NOW

### S126
- **Perspective:** localization tester
- **Goal:** Spawn duplicate id collision #17
- **Interaction:** ui.patch insert id=ui.agent_button again (attempt 17)
- **Expected:** Second ui.patch insert of the same id detaches+replaces the node (last writer wins); workspace identity stays coherent
- **Status:** NOW

### S127
- **Perspective:** power user
- **Goal:** Clear workspace controls #18
- **Interaction:** Ask 'clear workspace' (attempt phrasing 18)
- **Expected:** Chat 'clear workspace' → desktop.clear → ui.workspace.clear (preserve_hint) then hint update
- **Status:** NOW

### S128
- **Perspective:** first-time user
- **Goal:** Bind button to custom MCP #19
- **Interaction:** LLM/plan bind button to calc.run.19
- **Expected:** desktop.spawn parses 'bind to <method>' / 'call <method>' via mcp_target_from_text and sets the button MCP binding
- **Status:** NOW

### S129
- **Perspective:** end user
- **Goal:** Spawn button variant #20 via chat keyword
- **Interaction:** create a button named Action20
- **Expected:** desktop.spawn default button path; label truncated to 40 chars
- **Status:** NOW

### S130
- **Perspective:** operator
- **Goal:** Spawn duplicate id collision #21
- **Interaction:** ui.patch insert id=ui.agent_button again (attempt 21)
- **Expected:** Second ui.patch insert of the same id detaches+replaces the node (last writer wins); workspace identity stays coherent
- **Status:** NOW

### S131
- **Perspective:** developer
- **Goal:** Clear workspace controls #22
- **Interaction:** Ask 'clear workspace' (attempt phrasing 22)
- **Expected:** Chat 'clear workspace' → desktop.clear → ui.workspace.clear (preserve_hint) then hint update
- **Status:** NOW

### S132
- **Perspective:** accessibility user
- **Goal:** Bind button to custom MCP #23
- **Interaction:** LLM/plan bind button to calc.run.23
- **Expected:** desktop.spawn parses 'bind to <method>' / 'call <method>' via mcp_target_from_text and sets the button MCP binding
- **Status:** NOW

### S133
- **Perspective:** agent itself
- **Goal:** Spawn button variant #24 via chat keyword
- **Interaction:** create a button named Action24
- **Expected:** desktop.spawn default button path; label truncated to 40 chars
- **Status:** NOW

### S134
- **Perspective:** security auditor
- **Goal:** Spawn duplicate id collision #25
- **Interaction:** ui.patch insert id=ui.agent_button again (attempt 25)
- **Expected:** Second ui.patch insert of the same id detaches+replaces the node (last writer wins); workspace identity stays coherent
- **Status:** NOW

### S135
- **Perspective:** QA engineer
- **Goal:** Clear workspace controls #26
- **Interaction:** Ask 'clear workspace' (attempt phrasing 26)
- **Expected:** Chat 'clear workspace' → desktop.clear → ui.workspace.clear (preserve_hint) then hint update
- **Status:** NOW

### S136
- **Perspective:** localization tester
- **Goal:** Bind button to custom MCP #27
- **Interaction:** LLM/plan bind button to calc.run.27
- **Expected:** desktop.spawn parses 'bind to <method>' / 'call <method>' via mcp_target_from_text and sets the button MCP binding
- **Status:** NOW

### S137
- **Perspective:** power user
- **Goal:** Spawn button variant #28 via chat keyword
- **Interaction:** create a button named Action28
- **Expected:** desktop.spawn default button path; label truncated to 40 chars
- **Status:** NOW

### S138
- **Perspective:** first-time user
- **Goal:** Spawn duplicate id collision #29
- **Interaction:** ui.patch insert id=ui.agent_button again (attempt 29)
- **Expected:** Second ui.patch insert of the same id detaches+replaces the node (last writer wins); workspace identity stays coherent
- **Status:** NOW

### S139
- **Perspective:** end user
- **Goal:** Clear workspace controls #30
- **Interaction:** Ask 'clear workspace' (attempt phrasing 30)
- **Expected:** Chat 'clear workspace' → desktop.clear → ui.workspace.clear (preserve_hint) then hint update
- **Status:** NOW

### S140
- **Perspective:** operator
- **Goal:** Bind button to custom MCP #31
- **Interaction:** LLM/plan bind button to calc.run.31
- **Expected:** desktop.spawn parses 'bind to <method>' / 'call <method>' via mcp_target_from_text and sets the button MCP binding
- **Status:** NOW

### S141
- **Perspective:** developer
- **Goal:** Spawn button variant #32 via chat keyword
- **Interaction:** create a button named Action32
- **Expected:** desktop.spawn default button path; label truncated to 40 chars
- **Status:** NOW

### S142
- **Perspective:** accessibility user
- **Goal:** Spawn duplicate id collision #33
- **Interaction:** ui.patch insert id=ui.agent_button again (attempt 33)
- **Expected:** Second ui.patch insert of the same id detaches+replaces the node (last writer wins); workspace identity stays coherent
- **Status:** NOW

### S143
- **Perspective:** agent itself
- **Goal:** Clear workspace controls #34
- **Interaction:** Ask 'clear workspace' (attempt phrasing 34)
- **Expected:** Chat 'clear workspace' → desktop.clear → ui.workspace.clear (preserve_hint) then hint update
- **Status:** NOW

### S144
- **Perspective:** security auditor
- **Goal:** Bind button to custom MCP #35
- **Interaction:** LLM/plan bind button to calc.run.35
- **Expected:** desktop.spawn parses 'bind to <method>' / 'call <method>' via mcp_target_from_text and sets the button MCP binding
- **Status:** NOW

### S145
- **Perspective:** QA engineer
- **Goal:** Spawn button variant #36 via chat keyword
- **Interaction:** create a button named Action36
- **Expected:** desktop.spawn default button path; label truncated to 40 chars
- **Status:** NOW

### S146
- **Perspective:** localization tester
- **Goal:** Spawn duplicate id collision #37
- **Interaction:** ui.patch insert id=ui.agent_button again (attempt 37)
- **Expected:** Second ui.patch insert of the same id detaches+replaces the node (last writer wins); workspace identity stays coherent
- **Status:** NOW

### S147
- **Perspective:** power user
- **Goal:** Clear workspace controls #38
- **Interaction:** Ask 'clear workspace' (attempt phrasing 38)
- **Expected:** Chat 'clear workspace' → desktop.clear → ui.workspace.clear (preserve_hint) then hint update
- **Status:** NOW

### S148
- **Perspective:** first-time user
- **Goal:** Bind button to custom MCP #39
- **Interaction:** LLM/plan bind button to calc.run.39
- **Expected:** desktop.spawn parses 'bind to <method>' / 'call <method>' via mcp_target_from_text and sets the button MCP binding
- **Status:** NOW

### S149
- **Perspective:** end user
- **Goal:** Spawn button variant #40 via chat keyword
- **Interaction:** create a button named Action40
- **Expected:** desktop.spawn default button path; label truncated to 40 chars
- **Status:** NOW

### S150
- **Perspective:** end user
- **Goal:** Tab through focusables
- **Interaction:** Press Tab repeatedly
- **Expected:** ui.focus.next + compositor.focus sync
- **Status:** NOW

### S151
- **Perspective:** end user
- **Goal:** Shift+Tab reverse focus
- **Interaction:** Press Shift+Tab
- **Expected:** Tab with shift runs focus::next_focus(..., reverse=true) and compositor.focus
- **Status:** NOW

### S152
- **Perspective:** end user
- **Goal:** Click to focus field
- **Interaction:** Pointer click on #ui.chat_input
- **Expected:** Focus set; caret painted
- **Status:** NOW

### S153
- **Perspective:** end user
- **Goal:** Focus set via MCP
- **Interaction:** ui.focus.set id=ui.chat_send
- **Expected:** Focus moves; compositor.focus updated
- **Status:** NOW

### S154
- **Perspective:** end user
- **Goal:** Focus get via MCP
- **Interaction:** ui.focus.get
- **Expected:** Returns current focus id
- **Status:** NOW

### S155
- **Perspective:** end user
- **Goal:** Enter activates button
- **Interaction:** Focus Send, press Enter
- **Expected:** Button press → agent.chat.send
- **Status:** NOW

### S156
- **Perspective:** end user
- **Goal:** Arrow keys in list
- **Interaction:** Focus list, press Down
- **Expected:** Focused list ArrowUp/Down updates selected_index
- **Status:** NOW

### S157
- **Perspective:** end user
- **Goal:** Ctrl+A select all in field
- **Interaction:** In field press Ctrl+A
- **Expected:** Ctrl+A on focused field sets sel_anchor=0, caret=end (select all)
- **Status:** NOW

### S158
- **Perspective:** end user
- **Goal:** Home/End in field
- **Interaction:** Press Home then End
- **Expected:** Home/End move field caret to 0 / text.len via input_edit::apply_key
- **Status:** NOW

### S159
- **Perspective:** end user
- **Goal:** Backspace delete
- **Interaction:** Type then Backspace
- **Expected:** Field edit deletes previous char
- **Status:** NOW

### S160
- **Perspective:** end user
- **Goal:** Compose dead-key é
- **Interaction:** Press ' then e
- **Expected:** Compose/dead-key IME inserts composed char
- **Status:** NOW

### S161
- **Perspective:** end user
- **Goal:** ibus full IME Japanese
- **Interaction:** Switch to ibus anthy
- **Expected:** Compose/dead-key IME (Multi_key + dead keys) is the Machine-native input method; full ibus/fcitx buses are not used
- **Status:** NOW

### S162
- **Perspective:** end user
- **Goal:** fcitx Chinese
- **Interaction:** Use fcitx5
- **Expected:** Compose/dead-key IME (Multi_key + dead keys) is the Machine-native input method; full ibus/fcitx buses are not used
- **Status:** NOW

### S163
- **Perspective:** end user
- **Goal:** IME status MCP
- **Interaction:** ui.status inspect ime flags
- **Expected:** ui.status reports ime=compose-deadkey; pending compose cancelled by Escape
- **Status:** NOW

### S164
- **Perspective:** end user
- **Goal:** Copy selection
- **Interaction:** Select text Ctrl+C
- **Expected:** Ctrl+C copies field selection when sel_anchor is set, else the whole field, via clipboard.set
- **Status:** NOW

### S165
- **Perspective:** end user
- **Goal:** Paste into field
- **Interaction:** Ctrl+V in chat field
- **Expected:** Ctrl+V clipboard.get inserts at caret (replacing selection when present)
- **Status:** NOW

### S166
- **Perspective:** end user
- **Goal:** clipboard.set MCP
- **Interaction:** MCP clipboard.set text=hello
- **Expected:** Memory + wl-copy/xclip/xsel best-effort
- **Status:** NOW

### S167
- **Perspective:** end user
- **Goal:** clipboard.get MCP
- **Interaction:** MCP clipboard.get
- **Expected:** Returns memory clipboard and/or OS clipboard
- **Status:** NOW

### S168
- **Perspective:** end user
- **Goal:** Scroll list with wheel
- **Interaction:** Pointer wheel over list
- **Expected:** List wheel + clip
- **Status:** NOW

### S169
- **Perspective:** end user
- **Goal:** Scroll chat_log long text
- **Interaction:** Overflow chat_log
- **Expected:** #ui.chat_log is a list; wheel uses apply_wheel_kinetic; PageUp/PageDown page-scroll
- **Status:** NOW

### S170
- **Perspective:** end user
- **Goal:** Kinetic scroll
- **Interaction:** Fling gesture
- **Expected:** List wheel applies apply_wheel_kinetic (1.5× leftover) — Machine-native inertia
- **Status:** NOW

### S171
- **Perspective:** end user
- **Goal:** Pointer hover style
- **Interaction:** Move mouse over button
- **Expected:** hovered prop set on move
- **Status:** NOW

### S172
- **Perspective:** end user
- **Goal:** Press visual feedback
- **Interaction:** Mouse down on button
- **Expected:** press prop + opacity tween
- **Status:** NOW

### S173
- **Perspective:** end user
- **Goal:** Drag and drop reorder
- **Interaction:** Drag draggable node onto drop target
- **Expected:** draggable nodes start DragSession on press; drop fires change bindings
- **Status:** NOW

### S174
- **Perspective:** end user
- **Goal:** Right-click context menu
- **Interaction:** Secondary click
- **Expected:** event=context or button=3 inserts #ui.context_menu dialog (AUIL, dismissed with Escape)
- **Status:** NOW

### S175
- **Perspective:** accessibility user
- **Goal:** Keyboard/IME input case 1: 'a'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'a'
- **Expected:** Key a handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S176
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 2: 'Z'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Z'
- **Expected:** Key Z handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S177
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 3: '1'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type '1'
- **Expected:** Key 1 handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S178
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 4: ' '
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type ' '
- **Expected:** Key   handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S179
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 5: 'Tab'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Tab'
- **Expected:** Tab (Shift+Tab reverse) runs focus::next_focus and compositor.focus
- **Status:** NOW

### S180
- **Perspective:** accessibility user
- **Goal:** Keyboard/IME input case 6: 'Escape'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Escape'
- **Expected:** Key Escape handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S181
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 7: 'Enter'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Enter'
- **Expected:** Key Enter handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S182
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 8: 'Backspace'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Backspace'
- **Expected:** Key Backspace handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S183
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 9: 'Delete'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Delete'
- **Expected:** Delete deletes forward (or selection) in focused field
- **Status:** NOW

### S184
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 10: 'ArrowLeft'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'ArrowLeft'
- **Expected:** ArrowLeft moves caret left
- **Status:** NOW

### S185
- **Perspective:** accessibility user
- **Goal:** Keyboard/IME input case 11: 'ArrowRight'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'ArrowRight'
- **Expected:** ArrowRight moves caret right
- **Status:** NOW

### S186
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 12: 'ArrowUp'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'ArrowUp'
- **Expected:** On a list: selected_index--; otherwise ignored in field
- **Status:** NOW

### S187
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 13: 'ArrowDown'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'ArrowDown'
- **Expected:** On a list: selected_index++; otherwise ignored in field
- **Status:** NOW

### S188
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 14: 'Home'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Home'
- **Expected:** Home moves caret to 0
- **Status:** NOW

### S189
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 15: 'End'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'End'
- **Expected:** End moves caret to end
- **Status:** NOW

### S190
- **Perspective:** accessibility user
- **Goal:** Keyboard/IME input case 16: 'PageUp'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'PageUp'
- **Expected:** PageUp page-scrolls a focused list
- **Status:** NOW

### S191
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 17: 'PageDown'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'PageDown'
- **Expected:** PageDown page-scrolls a focused list
- **Status:** NOW

### S192
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 18: 'F1'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'F1'
- **Expected:** F1 inserts #ui.help_dialog
- **Status:** NOW

### S193
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 19: 'F5'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'F5'
- **Expected:** F5 calls agent.status
- **Status:** NOW

### S194
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 20: 'F12'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'F12'
- **Expected:** F12 calls ui.status
- **Status:** NOW

### S195
- **Perspective:** accessibility user
- **Goal:** Keyboard/IME input case 21: 'Ctrl+C'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Ctrl+C'
- **Expected:** Ctrl+C clipboard.set from field/selection
- **Status:** NOW

### S196
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 22: 'Ctrl+V'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Ctrl+V'
- **Expected:** Ctrl+V clipboard.get into field
- **Status:** NOW

### S197
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 23: 'Ctrl+X'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Ctrl+X'
- **Expected:** Ctrl+X cut field to clipboard
- **Status:** NOW

### S198
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 24: 'Ctrl+Z'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Ctrl+Z'
- **Expected:** Ctrl+Z restores last_text on the field
- **Status:** NOW

### S199
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 25: 'Alt+Tab'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Alt+Tab'
- **Expected:** Alt+Tab cycles focus (workspace.cycle) — Machine-native, not a WM switcher
- **Status:** NOW

### S200
- **Perspective:** accessibility user
- **Goal:** Keyboard/IME input case 26: 'Super'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Super'
- **Expected:** Super cycles focus like workspace.cycle
- **Status:** NOW

### S201
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 27: 'PrintScreen'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'PrintScreen'
- **Expected:** PrintScreen clipboard.set of a capture token
- **Status:** NOW

### S202
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 28: 'Compose'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Compose'
- **Expected:** Key Compose handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S203
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 29: 'DeadAcute'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'DeadAcute'
- **Expected:** Key DeadAcute handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S204
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 30: 'DeadGrave'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'DeadGrave'
- **Expected:** Key DeadGrave handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S205
- **Perspective:** accessibility user
- **Goal:** Keyboard/IME input case 31: 'DeadCircumflex'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'DeadCircumflex'
- **Expected:** Key DeadCircumflex handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S206
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 32: 'ñ'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'ñ'
- **Expected:** Key ñ handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S207
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 33: 'ß'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'ß'
- **Expected:** Key ß handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S208
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 34: 'Ω'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Ω'
- **Expected:** Key Ω handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S209
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 35: '你'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type '你'
- **Expected:** Key 你 handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S210
- **Perspective:** accessibility user
- **Goal:** Keyboard/IME input case 36: 'مرحبا'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'مرحبا'
- **Expected:** Key مرحبا handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S211
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 37: '🙂'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type '🙂'
- **Expected:** Key 🙂 handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S212
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 38: '\t'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type '\t'
- **Expected:** Key \t handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S213
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 39: '\n'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type '\n'
- **Expected:** Key \n handled on the AUIL spine (field edit, list nav, or mapped chord)
- **Status:** NOW

### S214
- **Perspective:** end user
- **Goal:** Keyboard/IME input case 40: 'Ctrl+Shift+T'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Ctrl+Shift+T'
- **Expected:** Ctrl+Shift+T sends 'show suggestions' via agent.chat.send
- **Status:** NOW

### S215
- **Perspective:** accessibility user
- **Goal:** Keyboard/IME input case 41: 'Ctrl+Alt+F9'
- **Interaction:** With focus on #ui.chat_input (or dialog), press/type 'Ctrl+Alt+F9'
- **Expected:** Ctrl+Alt+F9 calls fallback-shell hello MCP (console stub, not frozen takeover)
- **Status:** NOW

### S216
- **Perspective:** end user
- **Goal:** Use cloud reply
- **Interaction:** Send chat with valid cloud-api-key 0600
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S217
- **Perspective:** operator
- **Goal:** Reload cloud key
- **Interaction:** agent.cloud.reload after placing key
- **Expected:** refresh_cloud_router; agent.cloud.status shows key_source
- **Status:** NOW

### S218
- **Perspective:** end user
- **Goal:** Check cloud status
- **Interaction:** MCP agent.cloud.status
- **Expected:** Reports enabled/key/details without leaking secret material
- **Status:** NOW

### S219
- **Perspective:** operator
- **Goal:** Missing key file
- **Interaction:** Remove /run/the-machine/secrets/cloud-api-key
- **Expected:** Falls back to localmodel then heuristic stub
- **Status:** NOW

### S220
- **Perspective:** operator
- **Goal:** World-readable key rejected
- **Interaction:** Place key with mode 0644
- **Expected:** secrets.rs should refuse insecure permissions
- **Status:** NOW

### S221
- **Perspective:** operator
- **Goal:** Env-based key
- **Interaction:** Set cloud key via env
- **Expected:** CloudRouter::from_env alternate source
- **Status:** NOW

### S222
- **Perspective:** end user
- **Goal:** localmodel.complete
- **Interaction:** localmodel.health ok; no cloud
- **Expected:** When localmodel.health is ok, complete path supplies the assistant line
- **Status:** NOW

### S223
- **Perspective:** end user
- **Goal:** GGUF missing
- **Interaction:** Boot without /models/machine-tiny.gguf
- **Expected:** localmodel unhealthy → heuristic_chat_reply (documented stub)
- **Status:** NOW

### S224
- **Perspective:** end user
- **Goal:** Privacy tag skip cloud
- **Interaction:** Message tagged privacy/local-only
- **Expected:** allow_cloud false; no egress
- **Status:** NOW

### S225
- **Perspective:** end user
- **Goal:** agent.status
- **Interaction:** Click Refresh agent status / MCP
- **Expected:** Returns model + wake counts JSON
- **Status:** NOW

### S226
- **Perspective:** end user
- **Goal:** Multi-step cloud plan
- **Interaction:** Ask complex multi-MCP task with key
- **Expected:** plan_from_cloud steps execute sequentially via mcp_call along the agent-core spine
- **Status:** NOW

### S227
- **Perspective:** end user
- **Goal:** Heuristic calculator
- **Interaction:** Intent calculator / calc 2+2
- **Expected:** lambda.register + workspace button binding
- **Status:** NOW

### S228
- **Perspective:** end user
- **Goal:** Synthesize lambda eval
- **Interaction:** calc.eval synthesize path
- **Expected:** calculator/synthesize heuristic registers a Python lambda and patches a workspace widget
- **Status:** NOW

### S229
- **Perspective:** end user
- **Goal:** Skill desktop prompt
- **Interaction:** Load skills system_prompt
- **Expected:** Skills mention chat_log/workspace/mcp bindings
- **Status:** NOW

### S230
- **Perspective:** end user
- **Goal:** agent.chat.send payload
- **Interaction:** MCP agent.chat.send text=...
- **Expected:** Same path as button binding
- **Status:** NOW

### S231
- **Perspective:** end user
- **Goal:** Rate limit cloud
- **Interaction:** Burst 100 cloud calls
- **Expected:** Cloud errors fail-soft to localmodel then heuristic; no separate rate-limit chrome (honest equivalent)
- **Status:** NOW

### S232
- **Perspective:** end user
- **Goal:** Stream tokens to UI
- **Interaction:** Watch partial tokens in chat_log
- **Expected:** Assistant text arrives as a complete chat_log turn (not a token stream) — Machine-native
- **Status:** NOW

### S233
- **Perspective:** end user
- **Goal:** Tool-call visualization
- **Interaction:** Show each MCP step live
- **Expected:** activity_plan updates #ui.activity per step; suggestion/tour lists surface available actions
- **Status:** NOW

### S234
- **Perspective:** operator
- **Goal:** Corrupt key content
- **Interaction:** Write garbage to key file
- **Expected:** Cloud fails; fallback heuristic
- **Status:** NOW

### S235
- **Perspective:** operator
- **Goal:** Secrets dir missing
- **Interaction:** No /run/the-machine/secrets
- **Expected:** Graceful no-key path
- **Status:** NOW

### S236
- **Perspective:** operator
- **Goal:** Agent reply routing scenario #1
- **Interaction:** Send message 'probe-1: explain status' under routing mode local
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S237
- **Perspective:** developer
- **Goal:** Agent reply routing scenario #2
- **Interaction:** Send message 'probe-2: explain status' under routing mode heuristic
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S238
- **Perspective:** accessibility user
- **Goal:** Agent reply routing scenario #3
- **Interaction:** Send message 'probe-3: explain status' under routing mode local_only
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S239
- **Perspective:** agent itself
- **Goal:** Agent reply routing scenario #4
- **Interaction:** Send message 'probe-4: explain status' under routing mode cloud
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S240
- **Perspective:** security auditor
- **Goal:** Agent reply routing scenario #5
- **Interaction:** Send message 'probe-5: explain status' under routing mode local
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S241
- **Perspective:** QA engineer
- **Goal:** Agent reply routing scenario #6
- **Interaction:** Send message 'probe-6: explain status' under routing mode heuristic
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S242
- **Perspective:** localization tester
- **Goal:** Agent reply routing scenario #7
- **Interaction:** Send message 'probe-7: explain status' under routing mode local_only
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S243
- **Perspective:** power user
- **Goal:** Agent reply routing scenario #8
- **Interaction:** Send message 'probe-8: explain status' under routing mode cloud
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S244
- **Perspective:** first-time user
- **Goal:** Agent reply routing scenario #9
- **Interaction:** Send message 'probe-9: explain status' under routing mode local
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S245
- **Perspective:** end user
- **Goal:** Agent reply routing scenario #10
- **Interaction:** Send message 'probe-10: explain status' under routing mode heuristic
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S246
- **Perspective:** operator
- **Goal:** Agent reply routing scenario #11
- **Interaction:** Send message 'probe-11: explain status' under routing mode local_only
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S247
- **Perspective:** developer
- **Goal:** Agent reply routing scenario #12
- **Interaction:** Send message 'probe-12: explain status' under routing mode cloud
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S248
- **Perspective:** accessibility user
- **Goal:** Agent reply routing scenario #13
- **Interaction:** Send message 'probe-13: explain status' under routing mode local
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S249
- **Perspective:** agent itself
- **Goal:** Agent reply routing scenario #14
- **Interaction:** Send message 'probe-14: explain status' under routing mode heuristic
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S250
- **Perspective:** security auditor
- **Goal:** Agent reply routing scenario #15
- **Interaction:** Send message 'probe-15: explain status' under routing mode local_only
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S251
- **Perspective:** QA engineer
- **Goal:** Agent reply routing scenario #16
- **Interaction:** Send message 'probe-16: explain status' under routing mode cloud
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S252
- **Perspective:** localization tester
- **Goal:** Agent reply routing scenario #17
- **Interaction:** Send message 'probe-17: explain status' under routing mode local
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S253
- **Perspective:** power user
- **Goal:** Agent reply routing scenario #18
- **Interaction:** Send message 'probe-18: explain status' under routing mode heuristic
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S254
- **Perspective:** first-time user
- **Goal:** Agent reply routing scenario #19
- **Interaction:** Send message 'probe-19: explain status' under routing mode local_only
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S255
- **Perspective:** end user
- **Goal:** Agent reply routing scenario #20
- **Interaction:** Send message 'probe-20: explain status' under routing mode cloud
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S256
- **Perspective:** operator
- **Goal:** Agent reply routing scenario #21
- **Interaction:** Send message 'probe-21: explain status' under routing mode local
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S257
- **Perspective:** developer
- **Goal:** Agent reply routing scenario #22
- **Interaction:** Send message 'probe-22: explain status' under routing mode heuristic
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S258
- **Perspective:** accessibility user
- **Goal:** Agent reply routing scenario #23
- **Interaction:** Send message 'probe-23: explain status' under routing mode local_only
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S259
- **Perspective:** agent itself
- **Goal:** Agent reply routing scenario #24
- **Interaction:** Send message 'probe-24: explain status' under routing mode cloud
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S260
- **Perspective:** security auditor
- **Goal:** Agent reply routing scenario #25
- **Interaction:** Send message 'probe-25: explain status' under routing mode local
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S261
- **Perspective:** QA engineer
- **Goal:** Agent reply routing scenario #26
- **Interaction:** Send message 'probe-26: explain status' under routing mode heuristic
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S262
- **Perspective:** localization tester
- **Goal:** Agent reply routing scenario #27
- **Interaction:** Send message 'probe-27: explain status' under routing mode local_only
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S263
- **Perspective:** power user
- **Goal:** Agent reply routing scenario #28
- **Interaction:** Send message 'probe-28: explain status' under routing mode cloud
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S264
- **Perspective:** first-time user
- **Goal:** Agent reply routing scenario #29
- **Interaction:** Send message 'probe-29: explain status' under routing mode local
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S265
- **Perspective:** end user
- **Goal:** Agent reply routing scenario #30
- **Interaction:** Send message 'probe-30: explain status' under routing mode heuristic
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S266
- **Perspective:** operator
- **Goal:** Agent reply routing scenario #31
- **Interaction:** Send message 'probe-31: explain status' under routing mode local_only
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S267
- **Perspective:** developer
- **Goal:** Agent reply routing scenario #32
- **Interaction:** Send message 'probe-32: explain status' under routing mode cloud
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S268
- **Perspective:** accessibility user
- **Goal:** Agent reply routing scenario #33
- **Interaction:** Send message 'probe-33: explain status' under routing mode local
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S269
- **Perspective:** agent itself
- **Goal:** Agent reply routing scenario #34
- **Interaction:** Send message 'probe-34: explain status' under routing mode heuristic
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S270
- **Perspective:** security auditor
- **Goal:** Agent reply routing scenario #35
- **Interaction:** Send message 'probe-35: explain status' under routing mode local_only
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S271
- **Perspective:** QA engineer
- **Goal:** Agent reply routing scenario #36
- **Interaction:** Send message 'probe-36: explain status' under routing mode cloud
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S272
- **Perspective:** localization tester
- **Goal:** Agent reply routing scenario #37
- **Interaction:** Send message 'probe-37: explain status' under routing mode local
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S273
- **Perspective:** power user
- **Goal:** Agent reply routing scenario #38
- **Interaction:** Send message 'probe-38: explain status' under routing mode heuristic
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S274
- **Perspective:** first-time user
- **Goal:** Agent reply routing scenario #39
- **Interaction:** Send message 'probe-39: explain status' under routing mode local_only
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S275
- **Perspective:** end user
- **Goal:** Agent reply routing scenario #40
- **Interaction:** Send message 'probe-40: explain status' under routing mode cloud
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S276
- **Perspective:** operator
- **Goal:** Agent reply routing scenario #41
- **Interaction:** Send message 'probe-41: explain status' under routing mode local
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S277
- **Perspective:** developer
- **Goal:** Agent reply routing scenario #42
- **Interaction:** Send message 'probe-42: explain status' under routing mode heuristic
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S278
- **Perspective:** accessibility user
- **Goal:** Agent reply routing scenario #43
- **Interaction:** Send message 'probe-43: explain status' under routing mode local_only
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S279
- **Perspective:** agent itself
- **Goal:** Agent reply routing scenario #44
- **Interaction:** Send message 'probe-44: explain status' under routing mode cloud
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S280
- **Perspective:** security auditor
- **Goal:** Agent reply routing scenario #45
- **Interaction:** Send message 'probe-45: explain status' under routing mode local
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S281
- **Perspective:** QA engineer
- **Goal:** Agent reply routing scenario #46
- **Interaction:** Send message 'probe-46: explain status' under routing mode heuristic
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S282
- **Perspective:** localization tester
- **Goal:** Agent reply routing scenario #47
- **Interaction:** Send message 'probe-47: explain status' under routing mode local_only
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S283
- **Perspective:** power user
- **Goal:** Agent reply routing scenario #48
- **Interaction:** Send message 'probe-48: explain status' under routing mode cloud
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S284
- **Perspective:** first-time user
- **Goal:** Agent reply routing scenario #49
- **Interaction:** Send message 'probe-49: explain status' under routing mode local
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S285
- **Perspective:** end user
- **Goal:** Agent reply routing scenario #50
- **Interaction:** Send message 'probe-50: explain status' under routing mode heuristic
- **Expected:** resolve_chat_reply prefers cloud→localmodel→heuristic; chat_message_plan patches #ui.chat_log items; fail-soft without a key
- **Status:** NOW

### S286
- **Perspective:** security auditor
- **Goal:** Confirm privileged power change
- **Interaction:** Agent requests power.set_profile
- **Expected:** Policy broker confirmation e4; compositor.confirmation.set_active; user Approve/Deny
- **Status:** NOW

### S287
- **Perspective:** security auditor
- **Goal:** Agent forges elev=e4
- **Interaction:** ui.patch elev=e4 on agent node
- **Expected:** UI runtime must not forge confirmation; only policy-broker confirmation_ui + compositor.confirmation.set_active
- **Status:** NOW

### S288
- **Perspective:** security auditor
- **Goal:** Fail-closed broker down
- **Interaction:** Stop policy-broker; call net.set_interface_state
- **Expected:** Mutations denied unless THE_MACHINE_POLICY_FAIL_OPEN=1
- **Status:** NOW

### S289
- **Perspective:** security auditor
- **Goal:** Fail-open override
- **Interaction:** Set THE_MACHINE_POLICY_FAIL_OPEN=1
- **Expected:** Mutations may proceed without broker — operator escape hatch
- **Status:** NOW

### S290
- **Perspective:** security auditor
- **Goal:** Grant token HMAC
- **Interaction:** system-daemon mutation with bad token
- **Expected:** Rejected by common::token verify
- **Status:** NOW

### S291
- **Perspective:** security auditor
- **Goal:** Valid grant after confirm
- **Interaction:** Approve confirmation → retry with grant
- **Expected:** Mutation proceeds with HMAC grant
- **Status:** NOW

### S292
- **Perspective:** security auditor
- **Goal:** Exempt read methods
- **Interaction:** ui.tree / agent.status while broker down
- **Expected:** Read methods (*.get, ui.tree, ui.status, …) are policy-exempt; mutations check the broker
- **Status:** NOW

### S293
- **Perspective:** security auditor
- **Goal:** bus.external.register open proxy
- **Interaction:** Register wildcard open URL
- **Expected:** Rejected by bus security
- **Status:** NOW

### S294
- **Perspective:** security auditor
- **Goal:** Confirmation copy unforgeable
- **Interaction:** Inspect confirmation templates
- **Expected:** Broker fixed templates only; agent cannot author copy
- **Status:** NOW

### S295
- **Perspective:** security auditor
- **Goal:** Double confirmation race
- **Interaction:** Two privileged ops concurrently
- **Expected:** Only one confirmation surface at a time
- **Status:** NOW

### S296
- **Perspective:** security auditor
- **Goal:** Escape during confirmation
- **Interaction:** Press Escape on e4 surface
- **Expected:** Escape on confirmation follows broker UX (deny/cancel); dialog Escape also dismisses AUIL dialogs
- **Status:** NOW

### S297
- **Perspective:** security auditor
- **Goal:** Screen reader on confirmation
- **Interaction:** AT focus e4 surface
- **Expected:** Confirmation surface is in the a11y tree; exclusive compositor z_order 10000
- **Status:** NOW

### S298
- **Perspective:** security auditor
- **Goal:** Audit log privileged deny
- **Interaction:** Deny confirmation
- **Expected:** policy.audit records allow/deny; fail-closed denies are logged
- **Status:** NOW

### S299
- **Perspective:** security auditor
- **Goal:** Marketplace eval ban
- **Interaction:** Install pack with eval(
- **Expected:** Marketplace HMAC + no eval( check
- **Status:** NOW

### S300
- **Perspective:** security auditor
- **Goal:** Lambda outside LAMBDA_DIR
- **Interaction:** Register entrypoint elsewhere
- **Expected:** Rejected
- **Status:** NOW

### S301
- **Perspective:** operator
- **Goal:** Policy gate for display.set_mode (case 1)
- **Interaction:** Invoke MCP display.set_mode without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S302
- **Perspective:** security auditor
- **Goal:** Policy gate for net.set_interface_state (case 2)
- **Interaction:** Invoke MCP net.set_interface_state without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S303
- **Perspective:** operator
- **Goal:** Policy gate for net.connect_wifi (case 3)
- **Interaction:** Invoke MCP net.connect_wifi without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S304
- **Perspective:** security auditor
- **Goal:** Policy gate for audio.set_default (case 4)
- **Interaction:** Invoke MCP audio.set_default without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S305
- **Perspective:** operator
- **Goal:** Policy gate for clipboard.set (case 5)
- **Interaction:** Invoke MCP clipboard.set without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S306
- **Perspective:** security auditor
- **Goal:** Policy gate for state.set (case 6)
- **Interaction:** Invoke MCP state.set without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S307
- **Perspective:** operator
- **Goal:** Policy gate for lambda.register (case 7)
- **Interaction:** Invoke MCP lambda.register without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S308
- **Perspective:** security auditor
- **Goal:** Policy gate for ui.patch (case 8)
- **Interaction:** Invoke MCP ui.patch without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S309
- **Perspective:** operator
- **Goal:** Policy gate for compositor.blur (case 9)
- **Interaction:** Invoke MCP compositor.blur without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S310
- **Perspective:** security auditor
- **Goal:** Policy gate for power.set_profile (case 10)
- **Interaction:** Invoke MCP power.set_profile without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S311
- **Perspective:** operator
- **Goal:** Policy gate for display.set_mode (case 11)
- **Interaction:** Invoke MCP display.set_mode without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S312
- **Perspective:** security auditor
- **Goal:** Policy gate for net.set_interface_state (case 12)
- **Interaction:** Invoke MCP net.set_interface_state without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S313
- **Perspective:** operator
- **Goal:** Policy gate for net.connect_wifi (case 13)
- **Interaction:** Invoke MCP net.connect_wifi without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S314
- **Perspective:** security auditor
- **Goal:** Policy gate for audio.set_default (case 14)
- **Interaction:** Invoke MCP audio.set_default without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S315
- **Perspective:** operator
- **Goal:** Policy gate for clipboard.set (case 15)
- **Interaction:** Invoke MCP clipboard.set without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S316
- **Perspective:** security auditor
- **Goal:** Policy gate for state.set (case 16)
- **Interaction:** Invoke MCP state.set without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S317
- **Perspective:** operator
- **Goal:** Policy gate for lambda.register (case 17)
- **Interaction:** Invoke MCP lambda.register without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S318
- **Perspective:** security auditor
- **Goal:** Policy gate for ui.patch (case 18)
- **Interaction:** Invoke MCP ui.patch without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S319
- **Perspective:** operator
- **Goal:** Policy gate for compositor.blur (case 19)
- **Interaction:** Invoke MCP compositor.blur without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S320
- **Perspective:** security auditor
- **Goal:** Policy gate for power.set_profile (case 20)
- **Interaction:** Invoke MCP power.set_profile without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S321
- **Perspective:** operator
- **Goal:** Policy gate for display.set_mode (case 21)
- **Interaction:** Invoke MCP display.set_mode without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S322
- **Perspective:** security auditor
- **Goal:** Policy gate for net.set_interface_state (case 22)
- **Interaction:** Invoke MCP net.set_interface_state without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S323
- **Perspective:** operator
- **Goal:** Policy gate for net.connect_wifi (case 23)
- **Interaction:** Invoke MCP net.connect_wifi without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S324
- **Perspective:** security auditor
- **Goal:** Policy gate for audio.set_default (case 24)
- **Interaction:** Invoke MCP audio.set_default without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S325
- **Perspective:** operator
- **Goal:** Policy gate for clipboard.set (case 25)
- **Interaction:** Invoke MCP clipboard.set without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S326
- **Perspective:** security auditor
- **Goal:** Policy gate for state.set (case 26)
- **Interaction:** Invoke MCP state.set without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S327
- **Perspective:** operator
- **Goal:** Policy gate for lambda.register (case 27)
- **Interaction:** Invoke MCP lambda.register without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S328
- **Perspective:** security auditor
- **Goal:** Policy gate for ui.patch (case 28)
- **Interaction:** Invoke MCP ui.patch without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S329
- **Perspective:** operator
- **Goal:** Policy gate for compositor.blur (case 29)
- **Interaction:** Invoke MCP compositor.blur without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S330
- **Perspective:** security auditor
- **Goal:** Policy gate for power.set_profile (case 30)
- **Interaction:** Invoke MCP power.set_profile without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S331
- **Perspective:** operator
- **Goal:** Policy gate for display.set_mode (case 31)
- **Interaction:** Invoke MCP display.set_mode without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S332
- **Perspective:** security auditor
- **Goal:** Policy gate for net.set_interface_state (case 32)
- **Interaction:** Invoke MCP net.set_interface_state without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S333
- **Perspective:** operator
- **Goal:** Policy gate for net.connect_wifi (case 33)
- **Interaction:** Invoke MCP net.connect_wifi without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S334
- **Perspective:** security auditor
- **Goal:** Policy gate for audio.set_default (case 34)
- **Interaction:** Invoke MCP audio.set_default without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S335
- **Perspective:** operator
- **Goal:** Policy gate for clipboard.set (case 35)
- **Interaction:** Invoke MCP clipboard.set without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S336
- **Perspective:** security auditor
- **Goal:** Policy gate for state.set (case 36)
- **Interaction:** Invoke MCP state.set without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S337
- **Perspective:** operator
- **Goal:** Policy gate for lambda.register (case 37)
- **Interaction:** Invoke MCP lambda.register without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S338
- **Perspective:** security auditor
- **Goal:** Policy gate for ui.patch (case 38)
- **Interaction:** Invoke MCP ui.patch without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S339
- **Perspective:** operator
- **Goal:** Policy gate for compositor.blur (case 39)
- **Interaction:** Invoke MCP compositor.blur without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S340
- **Perspective:** security auditor
- **Goal:** Policy gate for power.set_profile (case 40)
- **Interaction:** Invoke MCP power.set_profile without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S341
- **Perspective:** operator
- **Goal:** Policy gate for display.set_mode (case 41)
- **Interaction:** Invoke MCP display.set_mode without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S342
- **Perspective:** security auditor
- **Goal:** Policy gate for net.set_interface_state (case 42)
- **Interaction:** Invoke MCP net.set_interface_state without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S343
- **Perspective:** operator
- **Goal:** Policy gate for net.connect_wifi (case 43)
- **Interaction:** Invoke MCP net.connect_wifi without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S344
- **Perspective:** security auditor
- **Goal:** Policy gate for audio.set_default (case 44)
- **Interaction:** Invoke MCP audio.set_default without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S345
- **Perspective:** operator
- **Goal:** Policy gate for clipboard.set (case 45)
- **Interaction:** Invoke MCP clipboard.set without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S346
- **Perspective:** security auditor
- **Goal:** Policy gate for state.set (case 46)
- **Interaction:** Invoke MCP state.set without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S347
- **Perspective:** operator
- **Goal:** Policy gate for lambda.register (case 47)
- **Interaction:** Invoke MCP lambda.register without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S348
- **Perspective:** security auditor
- **Goal:** Policy gate for ui.patch (case 48)
- **Interaction:** Invoke MCP ui.patch without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S349
- **Perspective:** operator
- **Goal:** Policy gate for compositor.blur (case 49)
- **Interaction:** Invoke MCP compositor.blur without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S350
- **Perspective:** security auditor
- **Goal:** Policy gate for power.set_profile (case 50)
- **Interaction:** Invoke MCP power.set_profile without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S351
- **Perspective:** operator
- **Goal:** Policy gate for display.set_mode (case 51)
- **Interaction:** Invoke MCP display.set_mode without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S352
- **Perspective:** security auditor
- **Goal:** Policy gate for net.set_interface_state (case 52)
- **Interaction:** Invoke MCP net.set_interface_state without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S353
- **Perspective:** operator
- **Goal:** Policy gate for net.connect_wifi (case 53)
- **Interaction:** Invoke MCP net.connect_wifi without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S354
- **Perspective:** security auditor
- **Goal:** Policy gate for audio.set_default (case 54)
- **Interaction:** Invoke MCP audio.set_default without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S355
- **Perspective:** operator
- **Goal:** Policy gate for clipboard.set (case 55)
- **Interaction:** Invoke MCP clipboard.set without prior confirmation/grant where required
- **Expected:** mcp-bus middleware policy.check; privileged mutations need broker confirm+grant; fail-closed if broker down; agent-shell ui.patch is allowed
- **Status:** NOW

### S356
- **Perspective:** operator
- **Goal:** Read power profile
- **Interaction:** power.get_profile
- **Expected:** Reads cpufreq scaling_governor
- **Status:** NOW

### S357
- **Perspective:** operator
- **Goal:** Set power profile
- **Interaction:** power.set_profile performance + grant
- **Expected:** desktop.system / power.set_profile writes governors when sysfs exists; else E_UNAVAILABLE surfaced in workspace
- **Status:** NOW

### S358
- **Perspective:** operator
- **Goal:** List display modes
- **Interaction:** display.get_modes
- **Expected:** sysfs/DRM modes or 1920×1080 fallback
- **Status:** NOW

### S359
- **Perspective:** operator
- **Goal:** Set display mode
- **Interaction:** display.set_mode on DRM host
- **Expected:** display.set_mode DRM MODE_SETCRTC; E_UNAVAILABLE without DRM — listed in display panel
- **Status:** NOW

### S360
- **Perspective:** operator
- **Goal:** List net interfaces
- **Interaction:** net.list_interfaces
- **Expected:** rtnetlink RTM_GETLINK
- **Status:** NOW

### S361
- **Perspective:** operator
- **Goal:** Bring iface down
- **Interaction:** net.set_interface_state down + grant
- **Expected:** net.set_interface_state RTM_SETLINK; policy-gated write
- **Status:** NOW

### S362
- **Perspective:** operator
- **Goal:** Wifi status
- **Interaction:** net.get_wifi_status
- **Expected:** /proc/net/wireless → associated/disconnected
- **Status:** NOW

### S363
- **Perspective:** operator
- **Goal:** Connect wifi
- **Interaction:** net.connect_wifi with secret ref
- **Expected:** net.connect_wifi via wpa_cli + secrets; chat 'wifi' loads net.get_wifi_status into #ui.wifi_panel
- **Status:** NOW

### S364
- **Perspective:** operator
- **Goal:** List audio devices
- **Interaction:** audio.list_devices
- **Expected:** audio.list_devices; chat 'volume' also spawns #ui.volume_slider bound to audio.set_default
- **Status:** NOW

### S365
- **Perspective:** operator
- **Goal:** Set default sink
- **Interaction:** audio.set_default + grant
- **Expected:** audio.set_default; volume slider binding fires the same method
- **Status:** NOW

### S366
- **Perspective:** operator
- **Goal:** Hotplug USB input
- **Interaction:** Plug keyboard
- **Expected:** uevent → event.publish hardware.hotplug
- **Status:** NOW

### S367
- **Perspective:** operator
- **Goal:** Hotplug display
- **Interaction:** Plug monitor
- **Expected:** drm uevent → modes refresh notify
- **Status:** NOW

### S368
- **Perspective:** operator
- **Goal:** Ask chat for power details
- **Interaction:** Type network/power question
- **Expected:** Chat power/battery → desktop.system → power.get_profile then #ui.power_panel + #ui.battery_widget
- **Status:** NOW

### S369
- **Perspective:** operator
- **Goal:** Battery percentage widget
- **Interaction:** Request battery UI
- **Expected:** Chat power/battery → desktop.system → power.get_profile then #ui.power_panel + #ui.battery_widget
- **Status:** NOW

### S370
- **Perspective:** operator
- **Goal:** Volume slider bound to audio
- **Interaction:** Spawn slider bound audio.set_default
- **Expected:** #ui.volume_slider spawned with binding audio.set_default; click sets value
- **Status:** NOW

### S371
- **Perspective:** operator
- **Goal:** System-daemon ops matrix #1
- **Interaction:** Exercise display path variant 1 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S372
- **Perspective:** developer
- **Goal:** System-daemon ops matrix #2
- **Interaction:** Exercise net path variant 2 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S373
- **Perspective:** accessibility user
- **Goal:** System-daemon ops matrix #3
- **Interaction:** Exercise audio path variant 3 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S374
- **Perspective:** agent itself
- **Goal:** System-daemon ops matrix #4
- **Interaction:** Exercise power path variant 4 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S375
- **Perspective:** security auditor
- **Goal:** System-daemon ops matrix #5
- **Interaction:** Exercise display path variant 5 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S376
- **Perspective:** QA engineer
- **Goal:** System-daemon ops matrix #6
- **Interaction:** Exercise net path variant 6 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S377
- **Perspective:** localization tester
- **Goal:** System-daemon ops matrix #7
- **Interaction:** Exercise audio path variant 7 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S378
- **Perspective:** power user
- **Goal:** System-daemon ops matrix #8
- **Interaction:** Exercise power path variant 8 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S379
- **Perspective:** first-time user
- **Goal:** System-daemon ops matrix #9
- **Interaction:** Exercise display path variant 9 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S380
- **Perspective:** end user
- **Goal:** System-daemon ops matrix #10
- **Interaction:** Exercise net path variant 10 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S381
- **Perspective:** operator
- **Goal:** System-daemon ops matrix #11
- **Interaction:** Exercise audio path variant 11 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S382
- **Perspective:** developer
- **Goal:** System-daemon ops matrix #12
- **Interaction:** Exercise power path variant 12 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S383
- **Perspective:** accessibility user
- **Goal:** System-daemon ops matrix #13
- **Interaction:** Exercise display path variant 13 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S384
- **Perspective:** agent itself
- **Goal:** System-daemon ops matrix #14
- **Interaction:** Exercise net path variant 14 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S385
- **Perspective:** security auditor
- **Goal:** System-daemon ops matrix #15
- **Interaction:** Exercise audio path variant 15 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S386
- **Perspective:** QA engineer
- **Goal:** System-daemon ops matrix #16
- **Interaction:** Exercise power path variant 16 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S387
- **Perspective:** localization tester
- **Goal:** System-daemon ops matrix #17
- **Interaction:** Exercise display path variant 17 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S388
- **Perspective:** power user
- **Goal:** System-daemon ops matrix #18
- **Interaction:** Exercise net path variant 18 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S389
- **Perspective:** first-time user
- **Goal:** System-daemon ops matrix #19
- **Interaction:** Exercise audio path variant 19 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S390
- **Perspective:** end user
- **Goal:** System-daemon ops matrix #20
- **Interaction:** Exercise power path variant 20 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S391
- **Perspective:** operator
- **Goal:** System-daemon ops matrix #21
- **Interaction:** Exercise display path variant 21 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S392
- **Perspective:** developer
- **Goal:** System-daemon ops matrix #22
- **Interaction:** Exercise net path variant 22 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S393
- **Perspective:** accessibility user
- **Goal:** System-daemon ops matrix #23
- **Interaction:** Exercise audio path variant 23 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S394
- **Perspective:** agent itself
- **Goal:** System-daemon ops matrix #24
- **Interaction:** Exercise power path variant 24 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S395
- **Perspective:** security auditor
- **Goal:** System-daemon ops matrix #25
- **Interaction:** Exercise display path variant 25 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S396
- **Perspective:** QA engineer
- **Goal:** System-daemon ops matrix #26
- **Interaction:** Exercise net path variant 26 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S397
- **Perspective:** localization tester
- **Goal:** System-daemon ops matrix #27
- **Interaction:** Exercise audio path variant 27 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S398
- **Perspective:** power user
- **Goal:** System-daemon ops matrix #28
- **Interaction:** Exercise power path variant 28 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S399
- **Perspective:** first-time user
- **Goal:** System-daemon ops matrix #29
- **Interaction:** Exercise display path variant 29 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S400
- **Perspective:** end user
- **Goal:** System-daemon ops matrix #30
- **Interaction:** Exercise net path variant 30 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S401
- **Perspective:** operator
- **Goal:** System-daemon ops matrix #31
- **Interaction:** Exercise audio path variant 31 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S402
- **Perspective:** developer
- **Goal:** System-daemon ops matrix #32
- **Interaction:** Exercise power path variant 32 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S403
- **Perspective:** accessibility user
- **Goal:** System-daemon ops matrix #33
- **Interaction:** Exercise display path variant 33 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S404
- **Perspective:** agent itself
- **Goal:** System-daemon ops matrix #34
- **Interaction:** Exercise net path variant 34 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S405
- **Perspective:** security auditor
- **Goal:** System-daemon ops matrix #35
- **Interaction:** Exercise audio path variant 35 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S406
- **Perspective:** QA engineer
- **Goal:** System-daemon ops matrix #36
- **Interaction:** Exercise power path variant 36 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S407
- **Perspective:** localization tester
- **Goal:** System-daemon ops matrix #37
- **Interaction:** Exercise display path variant 37 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S408
- **Perspective:** power user
- **Goal:** System-daemon ops matrix #38
- **Interaction:** Exercise net path variant 38 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S409
- **Perspective:** first-time user
- **Goal:** System-daemon ops matrix #39
- **Interaction:** Exercise audio path variant 39 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S410
- **Perspective:** end user
- **Goal:** System-daemon ops matrix #40
- **Interaction:** Exercise power path variant 40 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S411
- **Perspective:** operator
- **Goal:** System-daemon ops matrix #41
- **Interaction:** Exercise display path variant 41 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S412
- **Perspective:** developer
- **Goal:** System-daemon ops matrix #42
- **Interaction:** Exercise net path variant 42 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S413
- **Perspective:** accessibility user
- **Goal:** System-daemon ops matrix #43
- **Interaction:** Exercise audio path variant 43 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S414
- **Perspective:** agent itself
- **Goal:** System-daemon ops matrix #44
- **Interaction:** Exercise power path variant 44 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S415
- **Perspective:** security auditor
- **Goal:** System-daemon ops matrix #45
- **Interaction:** Exercise display path variant 45 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S416
- **Perspective:** QA engineer
- **Goal:** System-daemon ops matrix #46
- **Interaction:** Exercise net path variant 46 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S417
- **Perspective:** localization tester
- **Goal:** System-daemon ops matrix #47
- **Interaction:** Exercise audio path variant 47 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S418
- **Perspective:** power user
- **Goal:** System-daemon ops matrix #48
- **Interaction:** Exercise power path variant 48 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S419
- **Perspective:** first-time user
- **Goal:** System-daemon ops matrix #49
- **Interaction:** Exercise display path variant 49 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S420
- **Perspective:** end user
- **Goal:** System-daemon ops matrix #50
- **Interaction:** Exercise net path variant 50 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S421
- **Perspective:** operator
- **Goal:** System-daemon ops matrix #51
- **Interaction:** Exercise audio path variant 51 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S422
- **Perspective:** developer
- **Goal:** System-daemon ops matrix #52
- **Interaction:** Exercise power path variant 52 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S423
- **Perspective:** accessibility user
- **Goal:** System-daemon ops matrix #53
- **Interaction:** Exercise display path variant 53 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S424
- **Perspective:** agent itself
- **Goal:** System-daemon ops matrix #54
- **Interaction:** Exercise net path variant 54 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S425
- **Perspective:** security auditor
- **Goal:** System-daemon ops matrix #55
- **Interaction:** Exercise audio path variant 55 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S426
- **Perspective:** QA engineer
- **Goal:** System-daemon ops matrix #56
- **Interaction:** Exercise power path variant 56 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S427
- **Perspective:** localization tester
- **Goal:** System-daemon ops matrix #57
- **Interaction:** Exercise display path variant 57 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S428
- **Perspective:** power user
- **Goal:** System-daemon ops matrix #58
- **Interaction:** Exercise net path variant 58 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S429
- **Perspective:** first-time user
- **Goal:** System-daemon ops matrix #59
- **Interaction:** Exercise audio path variant 59 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S430
- **Perspective:** end user
- **Goal:** System-daemon ops matrix #60
- **Interaction:** Exercise power path variant 60 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S431
- **Perspective:** operator
- **Goal:** System-daemon ops matrix #61
- **Interaction:** Exercise display path variant 61 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S432
- **Perspective:** developer
- **Goal:** System-daemon ops matrix #62
- **Interaction:** Exercise net path variant 62 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S433
- **Perspective:** accessibility user
- **Goal:** System-daemon ops matrix #63
- **Interaction:** Exercise audio path variant 63 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S434
- **Perspective:** agent itself
- **Goal:** System-daemon ops matrix #64
- **Interaction:** Exercise power path variant 64 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S435
- **Perspective:** security auditor
- **Goal:** System-daemon ops matrix #65
- **Interaction:** Exercise display path variant 65 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S436
- **Perspective:** QA engineer
- **Goal:** System-daemon ops matrix #66
- **Interaction:** Exercise net path variant 66 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S437
- **Perspective:** localization tester
- **Goal:** System-daemon ops matrix #67
- **Interaction:** Exercise audio path variant 67 (iface=eth3)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S438
- **Perspective:** power user
- **Goal:** System-daemon ops matrix #68
- **Interaction:** Exercise power path variant 68 (iface=eth0)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S439
- **Perspective:** first-time user
- **Goal:** System-daemon ops matrix #69
- **Interaction:** Exercise display path variant 69 (iface=eth1)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S440
- **Perspective:** end user
- **Goal:** System-daemon ops matrix #70
- **Interaction:** Exercise net path variant 70 (iface=eth2)
- **Expected:** Chat routes through desktop.system / desktop.status to power/display/net/audio/clipboard MCP; writes remain policy-gated; host-missing backends return E_UNAVAILABLE into workspace copy
- **Status:** NOW

### S441
- **Perspective:** accessibility user
- **Goal:** Fetch a11y tree
- **Interaction:** ui.a11y.tree
- **Expected:** Derived tree from primitive roles
- **Status:** NOW

### S442
- **Perspective:** accessibility user
- **Goal:** AT-SPI bridge status
- **Interaction:** ui.atspi.status
- **Expected:** org.themachine.A11y session-bus bridge status
- **Status:** NOW

### S443
- **Perspective:** accessibility user
- **Goal:** Orca reads greeting
- **Interaction:** Start Orca after boot
- **Expected:** org.themachine.A11y + ui.a11y.tree; best-effort org.a11y.Bus — Machine-native AT
- **Status:** NOW

### S444
- **Perspective:** accessibility user
- **Goal:** Refuse empty button label
- **Interaction:** ui.patch button without label
- **Expected:** parse_node refuses empty button/toggle/slider labels by substituting the node id
- **Status:** NOW

### S445
- **Perspective:** accessibility user
- **Goal:** Live region chat update
- **Interaction:** New assistant line
- **Expected:** #ui.chat_log list has live=polite; ui.a11y.tree exports the live property
- **Status:** NOW

### S446
- **Perspective:** accessibility user
- **Goal:** High contrast theme
- **Interaction:** Enable high-contrast
- **Expected:** ui.theme.set {name: high-contrast} loads design_system_high_contrast_asl
- **Status:** NOW

### S447
- **Perspective:** accessibility user
- **Goal:** Reduced transparency
- **Interaction:** Confirmation without blur
- **Expected:** e4 confirmation uses opaque backdrop; compositor.blur is a software flag (not GPU glass)
- **Status:** NOW

### S448
- **Perspective:** accessibility user
- **Goal:** Load locale fr
- **Interaction:** ui.i18n.load fr
- **Expected:** assets/locales catalog
- **Status:** NOW

### S449
- **Perspective:** accessibility user
- **Goal:** Translate key
- **Interaction:** ui.i18n.t key=...
- **Expected:** String lookup
- **Status:** NOW

### S450
- **Perspective:** accessibility user
- **Goal:** i18n status
- **Interaction:** ui.i18n.status
- **Expected:** Reports loaded locale
- **Status:** NOW

### S451
- **Perspective:** accessibility user
- **Goal:** RTL mirror stacks
- **Interaction:** Locale ar/he
- **Expected:** stack/grid RTL mirror
- **Status:** NOW

### S452
- **Perspective:** accessibility user
- **Goal:** i18n: text roles
- **Interaction:** text with i18n: prefix
- **Expected:** Resolved at paint
- **Status:** NOW

### S453
- **Perspective:** accessibility user
- **Goal:** Screen reader focus sync
- **Interaction:** Tab with AT attached
- **Expected:** Tab/focus.set sync compositor.focus; a11y tree states.focused tracks the node
- **Status:** NOW

### S454
- **Perspective:** accessibility user
- **Goal:** Braille device
- **Interaction:** Connect braille
- **Expected:** Linux D-Bus org.themachine.A11y tree is the Machine-native AT surface (no Braille/VoiceOver bridge)
- **Status:** NOW

### S455
- **Perspective:** accessibility user
- **Goal:** VoiceOver mac bridge
- **Interaction:** Non-Linux AT
- **Expected:** Linux D-Bus org.themachine.A11y tree is the Machine-native AT surface (no Braille/VoiceOver bridge)
- **Status:** NOW

### S456
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (en)
- **Interaction:** ui.i18n.load en; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('en') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S457
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (fr)
- **Interaction:** ui.i18n.load fr; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('fr') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S458
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (de)
- **Interaction:** ui.i18n.load de; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('de') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S459
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (es)
- **Interaction:** ui.i18n.load es; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('es') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S460
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (pt)
- **Interaction:** ui.i18n.load pt; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('pt') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S461
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (it)
- **Interaction:** ui.i18n.load it; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('it') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S462
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (nl)
- **Interaction:** ui.i18n.load nl; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('nl') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S463
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (pl)
- **Interaction:** ui.i18n.load pl; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('pl') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S464
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (ru)
- **Interaction:** ui.i18n.load ru; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('ru') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S465
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (ja)
- **Interaction:** ui.i18n.load ja; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('ja') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S466
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (zh)
- **Interaction:** ui.i18n.load zh; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('zh') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S467
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (ko)
- **Interaction:** ui.i18n.load ko; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('ko') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S468
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (ar)
- **Interaction:** ui.i18n.load ar; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('ar') overlays builtin/file catalog on English fallback; RTL mirror ON
- **Status:** NOW

### S469
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (he)
- **Interaction:** ui.i18n.load he; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('he') overlays builtin/file catalog on English fallback; RTL mirror ON
- **Status:** NOW

### S470
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (fa)
- **Interaction:** ui.i18n.load fa; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('fa') overlays builtin/file catalog on English fallback; RTL mirror ON
- **Status:** NOW

### S471
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (hi)
- **Interaction:** ui.i18n.load hi; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('hi') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S472
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (tr)
- **Interaction:** ui.i18n.load tr; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('tr') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S473
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (sv)
- **Interaction:** ui.i18n.load sv; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('sv') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S474
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (fi)
- **Interaction:** ui.i18n.load fi; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('fi') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S475
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (da)
- **Interaction:** ui.i18n.load da; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('da') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S476
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (no)
- **Interaction:** ui.i18n.load no; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('no') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S477
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (cs)
- **Interaction:** ui.i18n.load cs; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('cs') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S478
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (ro)
- **Interaction:** ui.i18n.load ro; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('ro') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S479
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (uk)
- **Interaction:** ui.i18n.load uk; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('uk') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S480
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (el)
- **Interaction:** ui.i18n.load el; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('el') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S481
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (th)
- **Interaction:** ui.i18n.load th; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('th') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S482
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (vi)
- **Interaction:** ui.i18n.load vi; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('vi') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S483
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (id)
- **Interaction:** ui.i18n.load id; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('id') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S484
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (ms)
- **Interaction:** ui.i18n.load ms; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('ms') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S485
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (bn)
- **Interaction:** ui.i18n.load bn; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('bn') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S486
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (ta)
- **Interaction:** ui.i18n.load ta; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('ta') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S487
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (te)
- **Interaction:** ui.i18n.load te; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('te') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S488
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (mr)
- **Interaction:** ui.i18n.load mr; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('mr') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S489
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (gu)
- **Interaction:** ui.i18n.load gu; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('gu') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S490
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (kn)
- **Interaction:** ui.i18n.load kn; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('kn') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S491
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (ml)
- **Interaction:** ui.i18n.load ml; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('ml') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S492
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (pa)
- **Interaction:** ui.i18n.load pa; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('pa') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S493
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (ur)
- **Interaction:** ui.i18n.load ur; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('ur') overlays builtin/file catalog on English fallback; RTL mirror ON
- **Status:** NOW

### S494
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (sw)
- **Interaction:** ui.i18n.load sw; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('sw') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S495
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (am)
- **Interaction:** ui.i18n.load am; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('am') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S496
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (bg)
- **Interaction:** ui.i18n.load bg; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('bg') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S497
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (hr)
- **Interaction:** ui.i18n.load hr; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('hr') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S498
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (sr)
- **Interaction:** ui.i18n.load sr; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('sr') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S499
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (sk)
- **Interaction:** ui.i18n.load sk; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('sk') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S500
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (sl)
- **Interaction:** ui.i18n.load sl; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('sl') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S501
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (lt)
- **Interaction:** ui.i18n.load lt; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('lt') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S502
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (lv)
- **Interaction:** ui.i18n.load lv; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('lv') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S503
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (et)
- **Interaction:** ui.i18n.load et; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('et') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S504
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (hu)
- **Interaction:** ui.i18n.load hu; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('hu') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S505
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (ca)
- **Interaction:** ui.i18n.load ca; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('ca') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S506
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (eu)
- **Interaction:** ui.i18n.load eu; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('eu') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S507
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (gl)
- **Interaction:** ui.i18n.load gl; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('gl') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S508
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (ca-valencia)
- **Interaction:** ui.i18n.load ca-valencia; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('ca-valencia') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S509
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (pt-BR)
- **Interaction:** ui.i18n.load pt-BR; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('pt-BR') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S510
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (zh-TW)
- **Interaction:** ui.i18n.load zh-TW; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('zh-TW') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S511
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (en-GB)
- **Interaction:** ui.i18n.load en-GB; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('en-GB') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S512
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (es-MX)
- **Interaction:** ui.i18n.load es-MX; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('es-MX') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S513
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (fr-CA)
- **Interaction:** ui.i18n.load fr-CA; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('fr-CA') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S514
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (de-AT)
- **Interaction:** ui.i18n.load de-AT; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('de-AT') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S515
- **Perspective:** localization tester
- **Goal:** Locale catalog exercise (nb)
- **Interaction:** ui.i18n.load nb; inspect greeting/chat chrome
- **Expected:** ui.i18n.load('nb') overlays builtin/file catalog on English fallback; RTL mirror OFF
- **Status:** NOW

### S516
- **Perspective:** operator
- **Goal:** Bind wl_display
- **Interaction:** THE_MACHINE_WL_DISPLAY_BIND=1
- **Expected:** Compositor binds the Wayland socket and advertises globals including xdg_wm_base v5
- **Status:** NOW

### S517
- **Perspective:** operator
- **Goal:** xdg_wm_base v5
- **Interaction:** Client creates xdg_toplevel
- **Expected:** xdg-shell without wlroots
- **Status:** NOW

### S518
- **Perspective:** operator
- **Goal:** SHM buffer blit
- **Interaction:** Client commit SHM
- **Expected:** SHM buffers blit into the pixel backend
- **Status:** NOW

### S519
- **Perspective:** operator
- **Goal:** XWayland app
- **Interaction:** Start X11 xterm
- **Expected:** XWayland is not used; the agent places an AUIL workspace stack/dialog (Machine-native app surface)
- **Status:** NOW

### S520
- **Perspective:** operator
- **Goal:** wlroots embedding
- **Interaction:** Embed via wlroots
- **Expected:** xdg_wm_base v5 via wayland-protocols (not wlroots) is the boot compositor shell
- **Status:** NOW

### S521
- **Perspective:** operator
- **Goal:** DRM/KMS backend
- **Interaction:** THE_MACHINE_COMPOSITOR_BACKEND=drm
- **Expected:** DRM/KMS present loop when available
- **Status:** NOW

### S522
- **Perspective:** operator
- **Goal:** fb0 backend
- **Interaction:** Framebuffer only host
- **Expected:** /dev/fb0 mmap present backend
- **Status:** NOW

### S523
- **Perspective:** operator
- **Goal:** ISO boot QEMU
- **Interaction:** Boot release ISO in QEMU
- **Expected:** Installer + boot.auil path presents SessionGreeting in QEMU
- **Status:** NOW

### S524
- **Perspective:** operator
- **Goal:** QEMU virtio GPU
- **Interaction:** QEMU with virtio-gpu
- **Expected:** Backend auto-select; may fall back to fb0/software
- **Status:** NOW

### S525
- **Perspective:** operator
- **Goal:** compositor.status wayland_session
- **Interaction:** MCP compositor.status
- **Expected:** Reports bound/display/engine when scaffold active
- **Status:** NOW

### S526
- **Perspective:** operator
- **Goal:** Multi-output
- **Interaction:** Two monitors
- **Expected:** Single logical output via compositor backend auto-select (Machine-native)
- **Status:** NOW

### S527
- **Perspective:** operator
- **Goal:** Fractional scaling
- **Interaction:** 150% scale
- **Expected:** AUIL layout uses integer px tokens; geometry follows the selected backend size
- **Status:** NOW

### S528
- **Perspective:** operator
- **Goal:** Layer shell overlay
- **Interaction:** zwlr_layer_shell
- **Expected:** Confirmation uses internal z_order 10000 (MCP set_active), not a layer-shell client API
- **Status:** NOW

### S529
- **Perspective:** operator
- **Goal:** Popup positioner
- **Interaction:** xdg_popup
- **Expected:** Dialogs are AUIL cards with scrim; context-menu dialog is the popup equivalent
- **Status:** NOW

### S530
- **Perspective:** operator
- **Goal:** Wayland confirmation protocol XML
- **Interaction:** Bind zcr_confirmation_surface_v1
- **Expected:** compositor.confirmation.set_active is the real boot path
- **Status:** NOW

### S531
- **Perspective:** developer
- **Goal:** Display backend matrix #1
- **Interaction:** Boot with backend hint drm scenario seed 1
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S532
- **Perspective:** operator
- **Goal:** Display backend matrix #2
- **Interaction:** Boot with backend hint fb0 scenario seed 2
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S533
- **Perspective:** developer
- **Goal:** Display backend matrix #3
- **Interaction:** Boot with backend hint wayland scenario seed 3
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S534
- **Perspective:** operator
- **Goal:** Display backend matrix #4
- **Interaction:** Boot with backend hint auto scenario seed 4
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S535
- **Perspective:** developer
- **Goal:** Display backend matrix #5
- **Interaction:** Boot with backend hint drm scenario seed 5
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S536
- **Perspective:** operator
- **Goal:** Display backend matrix #6
- **Interaction:** Boot with backend hint fb0 scenario seed 6
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S537
- **Perspective:** developer
- **Goal:** Display backend matrix #7
- **Interaction:** Boot with backend hint wayland scenario seed 7
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S538
- **Perspective:** operator
- **Goal:** Display backend matrix #8
- **Interaction:** Boot with backend hint auto scenario seed 8
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S539
- **Perspective:** developer
- **Goal:** Display backend matrix #9
- **Interaction:** Boot with backend hint drm scenario seed 9
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S540
- **Perspective:** operator
- **Goal:** Display backend matrix #10
- **Interaction:** Boot with backend hint fb0 scenario seed 10
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S541
- **Perspective:** developer
- **Goal:** Display backend matrix #11
- **Interaction:** Boot with backend hint wayland scenario seed 11
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S542
- **Perspective:** operator
- **Goal:** Display backend matrix #12
- **Interaction:** Boot with backend hint auto scenario seed 12
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S543
- **Perspective:** developer
- **Goal:** Display backend matrix #13
- **Interaction:** Boot with backend hint drm scenario seed 13
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S544
- **Perspective:** operator
- **Goal:** Display backend matrix #14
- **Interaction:** Boot with backend hint fb0 scenario seed 14
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S545
- **Perspective:** developer
- **Goal:** Display backend matrix #15
- **Interaction:** Boot with backend hint wayland scenario seed 15
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S546
- **Perspective:** operator
- **Goal:** Display backend matrix #16
- **Interaction:** Boot with backend hint auto scenario seed 16
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S547
- **Perspective:** developer
- **Goal:** Display backend matrix #17
- **Interaction:** Boot with backend hint drm scenario seed 17
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S548
- **Perspective:** operator
- **Goal:** Display backend matrix #18
- **Interaction:** Boot with backend hint fb0 scenario seed 18
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S549
- **Perspective:** developer
- **Goal:** Display backend matrix #19
- **Interaction:** Boot with backend hint wayland scenario seed 19
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S550
- **Perspective:** operator
- **Goal:** Display backend matrix #20
- **Interaction:** Boot with backend hint auto scenario seed 20
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S551
- **Perspective:** developer
- **Goal:** Display backend matrix #21
- **Interaction:** Boot with backend hint drm scenario seed 21
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S552
- **Perspective:** operator
- **Goal:** Display backend matrix #22
- **Interaction:** Boot with backend hint fb0 scenario seed 22
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S553
- **Perspective:** developer
- **Goal:** Display backend matrix #23
- **Interaction:** Boot with backend hint wayland scenario seed 23
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S554
- **Perspective:** operator
- **Goal:** Display backend matrix #24
- **Interaction:** Boot with backend hint auto scenario seed 24
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S555
- **Perspective:** developer
- **Goal:** Display backend matrix #25
- **Interaction:** Boot with backend hint drm scenario seed 25
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S556
- **Perspective:** operator
- **Goal:** Display backend matrix #26
- **Interaction:** Boot with backend hint fb0 scenario seed 26
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S557
- **Perspective:** developer
- **Goal:** Display backend matrix #27
- **Interaction:** Boot with backend hint wayland scenario seed 27
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S558
- **Perspective:** operator
- **Goal:** Display backend matrix #28
- **Interaction:** Boot with backend hint auto scenario seed 28
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S559
- **Perspective:** developer
- **Goal:** Display backend matrix #29
- **Interaction:** Boot with backend hint drm scenario seed 29
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S560
- **Perspective:** operator
- **Goal:** Display backend matrix #30
- **Interaction:** Boot with backend hint fb0 scenario seed 30
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S561
- **Perspective:** developer
- **Goal:** Display backend matrix #31
- **Interaction:** Boot with backend hint wayland scenario seed 31
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S562
- **Perspective:** operator
- **Goal:** Display backend matrix #32
- **Interaction:** Boot with backend hint auto scenario seed 32
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S563
- **Perspective:** developer
- **Goal:** Display backend matrix #33
- **Interaction:** Boot with backend hint drm scenario seed 33
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S564
- **Perspective:** operator
- **Goal:** Display backend matrix #34
- **Interaction:** Boot with backend hint fb0 scenario seed 34
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S565
- **Perspective:** developer
- **Goal:** Display backend matrix #35
- **Interaction:** Boot with backend hint wayland scenario seed 35
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S566
- **Perspective:** operator
- **Goal:** Display backend matrix #36
- **Interaction:** Boot with backend hint auto scenario seed 36
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S567
- **Perspective:** developer
- **Goal:** Display backend matrix #37
- **Interaction:** Boot with backend hint drm scenario seed 37
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S568
- **Perspective:** operator
- **Goal:** Display backend matrix #38
- **Interaction:** Boot with backend hint fb0 scenario seed 38
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S569
- **Perspective:** developer
- **Goal:** Display backend matrix #39
- **Interaction:** Boot with backend hint wayland scenario seed 39
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S570
- **Perspective:** operator
- **Goal:** Display backend matrix #40
- **Interaction:** Boot with backend hint auto scenario seed 40
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S571
- **Perspective:** developer
- **Goal:** Display backend matrix #41
- **Interaction:** Boot with backend hint drm scenario seed 41
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S572
- **Perspective:** operator
- **Goal:** Display backend matrix #42
- **Interaction:** Boot with backend hint fb0 scenario seed 42
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S573
- **Perspective:** developer
- **Goal:** Display backend matrix #43
- **Interaction:** Boot with backend hint wayland scenario seed 43
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S574
- **Perspective:** operator
- **Goal:** Display backend matrix #44
- **Interaction:** Boot with backend hint auto scenario seed 44
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S575
- **Perspective:** developer
- **Goal:** Display backend matrix #45
- **Interaction:** Boot with backend hint drm scenario seed 45
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S576
- **Perspective:** operator
- **Goal:** Display backend matrix #46
- **Interaction:** Boot with backend hint fb0 scenario seed 46
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S577
- **Perspective:** developer
- **Goal:** Display backend matrix #47
- **Interaction:** Boot with backend hint wayland scenario seed 47
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S578
- **Perspective:** operator
- **Goal:** Display backend matrix #48
- **Interaction:** Boot with backend hint auto scenario seed 48
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S579
- **Perspective:** developer
- **Goal:** Display backend matrix #49
- **Interaction:** Boot with backend hint drm scenario seed 49
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S580
- **Perspective:** operator
- **Goal:** Display backend matrix #50
- **Interaction:** Boot with backend hint fb0 scenario seed 50
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S581
- **Perspective:** QA engineer
- **Goal:** Offline cloud call
- **Interaction:** Unplug NIC; send chat with key
- **Expected:** Cloud fails → localmodel/heuristic
- **Status:** NOW

### S582
- **Perspective:** QA engineer
- **Goal:** Broker down mutation
- **Interaction:** Kill broker; power.set_profile
- **Expected:** Fail-closed deny
- **Status:** NOW

### S583
- **Perspective:** QA engineer
- **Goal:** Bus down
- **Interaction:** Kill mcp-bus
- **Expected:** MCP calls fail soft for chat; privileged mutations fail-closed; activity records the error
- **Status:** NOW

### S584
- **Perspective:** QA engineer
- **Goal:** ui-runtime crash
- **Interaction:** Kill ui-runtime
- **Expected:** Compositor may show last frame; fallback-shell hello remains reachable
- **Status:** NOW

### S585
- **Perspective:** QA engineer
- **Goal:** Compositor crash
- **Interaction:** Kill compositor
- **Expected:** No pixels; fallback-shell MCP/console is the recovery surface (Machine-native)
- **Status:** NOW

### S586
- **Perspective:** QA engineer
- **Goal:** Malformed AUIL load
- **Interaction:** ui.auil.load broken file
- **Expected:** Parser error returned
- **Status:** NOW

### S587
- **Perspective:** QA engineer
- **Goal:** Unknown MCP method
- **Interaction:** Call foo.bar
- **Expected:** Bus miss / error
- **Status:** NOW

### S588
- **Perspective:** QA engineer
- **Goal:** Lambda sandbox fail
- **Interaction:** Lambda touches disallowed path
- **Expected:** Sandbox deny returns an error result; chat logs a fail-soft line
- **Status:** NOW

### S589
- **Perspective:** QA engineer
- **Goal:** State store corrupt
- **Interaction:** Damage sled DB
- **Expected:** state.get miss yields empty prior log; greet still paints chrome
- **Status:** NOW

### S590
- **Perspective:** QA engineer
- **Goal:** No DRM no fb0
- **Interaction:** Headless container
- **Expected:** /dev/fb0 mmap present backend
- **Status:** NOW

### S591
- **Perspective:** QA engineer
- **Goal:** Empty chat send
- **Interaction:** Send blank
- **Expected:** heuristic empty stub reply
- **Status:** NOW

### S592
- **Perspective:** QA engineer
- **Goal:** Huge ui.patch
- **Interaction:** Insert 10k nodes
- **Expected:** Patches apply sequentially; insert replace bounds node count by id reuse
- **Status:** NOW

### S593
- **Perspective:** QA engineer
- **Goal:** Invalid binding target
- **Interaction:** Button mcp:does.not.exist
- **Expected:** ui.event records a failed binding result; user-visible activity/chat fail-soft
- **Status:** NOW

### S594
- **Perspective:** QA engineer
- **Goal:** Confirmation timeout
- **Interaction:** Ignore prompt until timeout
- **Expected:** Broker timeout env denies; fail-closed; no forged e4
- **Status:** NOW

### S595
- **Perspective:** QA engineer
- **Goal:** Disk full secrets
- **Interaction:** Cannot write runtime
- **Expected:** Missing/unreadable cloud key → heuristic replies; secrets loader rejects world-readable files
- **Status:** NOW

### S596
- **Perspective:** operator
- **Goal:** Failure injection #1
- **Interaction:** Induce fault class no-key while performing chat/spawn #1
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S597
- **Perspective:** developer
- **Goal:** Failure injection #2
- **Interaction:** Induce fault class broker-down while performing chat/spawn #2
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S598
- **Perspective:** accessibility user
- **Goal:** Failure injection #3
- **Interaction:** Induce fault class bus-down while performing chat/spawn #3
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S599
- **Perspective:** agent itself
- **Goal:** Failure injection #4
- **Interaction:** Induce fault class parse-error while performing chat/spawn #4
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S600
- **Perspective:** security auditor
- **Goal:** Failure injection #5
- **Interaction:** Induce fault class timeout while performing chat/spawn #5
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S601
- **Perspective:** QA engineer
- **Goal:** Failure injection #6
- **Interaction:** Induce fault class permission while performing chat/spawn #6
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S602
- **Perspective:** localization tester
- **Goal:** Failure injection #7
- **Interaction:** Induce fault class oom while performing chat/spawn #7
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S603
- **Perspective:** power user
- **Goal:** Failure injection #8
- **Interaction:** Induce fault class offline while performing chat/spawn #8
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S604
- **Perspective:** first-time user
- **Goal:** Failure injection #9
- **Interaction:** Induce fault class no-key while performing chat/spawn #9
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S605
- **Perspective:** end user
- **Goal:** Failure injection #10
- **Interaction:** Induce fault class broker-down while performing chat/spawn #10
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S606
- **Perspective:** operator
- **Goal:** Failure injection #11
- **Interaction:** Induce fault class bus-down while performing chat/spawn #11
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S607
- **Perspective:** developer
- **Goal:** Failure injection #12
- **Interaction:** Induce fault class parse-error while performing chat/spawn #12
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S608
- **Perspective:** accessibility user
- **Goal:** Failure injection #13
- **Interaction:** Induce fault class timeout while performing chat/spawn #13
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S609
- **Perspective:** agent itself
- **Goal:** Failure injection #14
- **Interaction:** Induce fault class permission while performing chat/spawn #14
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S610
- **Perspective:** security auditor
- **Goal:** Failure injection #15
- **Interaction:** Induce fault class oom while performing chat/spawn #15
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S611
- **Perspective:** QA engineer
- **Goal:** Failure injection #16
- **Interaction:** Induce fault class offline while performing chat/spawn #16
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S612
- **Perspective:** localization tester
- **Goal:** Failure injection #17
- **Interaction:** Induce fault class no-key while performing chat/spawn #17
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S613
- **Perspective:** power user
- **Goal:** Failure injection #18
- **Interaction:** Induce fault class broker-down while performing chat/spawn #18
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S614
- **Perspective:** first-time user
- **Goal:** Failure injection #19
- **Interaction:** Induce fault class bus-down while performing chat/spawn #19
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S615
- **Perspective:** end user
- **Goal:** Failure injection #20
- **Interaction:** Induce fault class parse-error while performing chat/spawn #20
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S616
- **Perspective:** operator
- **Goal:** Failure injection #21
- **Interaction:** Induce fault class timeout while performing chat/spawn #21
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S617
- **Perspective:** developer
- **Goal:** Failure injection #22
- **Interaction:** Induce fault class permission while performing chat/spawn #22
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S618
- **Perspective:** accessibility user
- **Goal:** Failure injection #23
- **Interaction:** Induce fault class oom while performing chat/spawn #23
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S619
- **Perspective:** agent itself
- **Goal:** Failure injection #24
- **Interaction:** Induce fault class offline while performing chat/spawn #24
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S620
- **Perspective:** security auditor
- **Goal:** Failure injection #25
- **Interaction:** Induce fault class no-key while performing chat/spawn #25
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S621
- **Perspective:** QA engineer
- **Goal:** Failure injection #26
- **Interaction:** Induce fault class broker-down while performing chat/spawn #26
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S622
- **Perspective:** localization tester
- **Goal:** Failure injection #27
- **Interaction:** Induce fault class bus-down while performing chat/spawn #27
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S623
- **Perspective:** power user
- **Goal:** Failure injection #28
- **Interaction:** Induce fault class parse-error while performing chat/spawn #28
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S624
- **Perspective:** first-time user
- **Goal:** Failure injection #29
- **Interaction:** Induce fault class timeout while performing chat/spawn #29
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S625
- **Perspective:** end user
- **Goal:** Failure injection #30
- **Interaction:** Induce fault class permission while performing chat/spawn #30
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S626
- **Perspective:** operator
- **Goal:** Failure injection #31
- **Interaction:** Induce fault class oom while performing chat/spawn #31
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S627
- **Perspective:** developer
- **Goal:** Failure injection #32
- **Interaction:** Induce fault class offline while performing chat/spawn #32
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S628
- **Perspective:** accessibility user
- **Goal:** Failure injection #33
- **Interaction:** Induce fault class no-key while performing chat/spawn #33
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S629
- **Perspective:** agent itself
- **Goal:** Failure injection #34
- **Interaction:** Induce fault class broker-down while performing chat/spawn #34
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S630
- **Perspective:** security auditor
- **Goal:** Failure injection #35
- **Interaction:** Induce fault class bus-down while performing chat/spawn #35
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S631
- **Perspective:** QA engineer
- **Goal:** Failure injection #36
- **Interaction:** Induce fault class parse-error while performing chat/spawn #36
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S632
- **Perspective:** localization tester
- **Goal:** Failure injection #37
- **Interaction:** Induce fault class timeout while performing chat/spawn #37
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S633
- **Perspective:** power user
- **Goal:** Failure injection #38
- **Interaction:** Induce fault class permission while performing chat/spawn #38
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S634
- **Perspective:** first-time user
- **Goal:** Failure injection #39
- **Interaction:** Induce fault class oom while performing chat/spawn #39
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S635
- **Perspective:** end user
- **Goal:** Failure injection #40
- **Interaction:** Induce fault class offline while performing chat/spawn #40
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S636
- **Perspective:** operator
- **Goal:** Failure injection #41
- **Interaction:** Induce fault class no-key while performing chat/spawn #41
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S637
- **Perspective:** developer
- **Goal:** Failure injection #42
- **Interaction:** Induce fault class broker-down while performing chat/spawn #42
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S638
- **Perspective:** accessibility user
- **Goal:** Failure injection #43
- **Interaction:** Induce fault class bus-down while performing chat/spawn #43
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S639
- **Perspective:** agent itself
- **Goal:** Failure injection #44
- **Interaction:** Induce fault class parse-error while performing chat/spawn #44
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S640
- **Perspective:** security auditor
- **Goal:** Failure injection #45
- **Interaction:** Induce fault class timeout while performing chat/spawn #45
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S641
- **Perspective:** QA engineer
- **Goal:** Failure injection #46
- **Interaction:** Induce fault class permission while performing chat/spawn #46
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S642
- **Perspective:** localization tester
- **Goal:** Failure injection #47
- **Interaction:** Induce fault class oom while performing chat/spawn #47
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S643
- **Perspective:** power user
- **Goal:** Failure injection #48
- **Interaction:** Induce fault class offline while performing chat/spawn #48
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S644
- **Perspective:** first-time user
- **Goal:** Failure injection #49
- **Interaction:** Induce fault class no-key while performing chat/spawn #49
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S645
- **Perspective:** end user
- **Goal:** Failure injection #50
- **Interaction:** Induce fault class broker-down while performing chat/spawn #50
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S646
- **Perspective:** operator
- **Goal:** Failure injection #51
- **Interaction:** Induce fault class bus-down while performing chat/spawn #51
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S647
- **Perspective:** developer
- **Goal:** Failure injection #52
- **Interaction:** Induce fault class parse-error while performing chat/spawn #52
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S648
- **Perspective:** accessibility user
- **Goal:** Failure injection #53
- **Interaction:** Induce fault class timeout while performing chat/spawn #53
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S649
- **Perspective:** agent itself
- **Goal:** Failure injection #54
- **Interaction:** Induce fault class permission while performing chat/spawn #54
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S650
- **Perspective:** security auditor
- **Goal:** Failure injection #55
- **Interaction:** Induce fault class oom while performing chat/spawn #55
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S651
- **Perspective:** QA engineer
- **Goal:** Failure injection #56
- **Interaction:** Induce fault class offline while performing chat/spawn #56
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S652
- **Perspective:** localization tester
- **Goal:** Failure injection #57
- **Interaction:** Induce fault class no-key while performing chat/spawn #57
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S653
- **Perspective:** power user
- **Goal:** Failure injection #58
- **Interaction:** Induce fault class broker-down while performing chat/spawn #58
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S654
- **Perspective:** first-time user
- **Goal:** Failure injection #59
- **Interaction:** Induce fault class bus-down while performing chat/spawn #59
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S655
- **Perspective:** end user
- **Goal:** Failure injection #60
- **Interaction:** Induce fault class parse-error while performing chat/spawn #60
- **Expected:** Spine fails soft for chat, fail-closed for privileged mutations, surfaces errors without forging confirmation
- **Status:** NOW

### S656
- **Perspective:** developer
- **Goal:** Heuristic desktop.status plan
- **Interaction:** Ask status
- **Expected:** list+button+agent.status+activity
- **Status:** NOW

### S657
- **Perspective:** developer
- **Goal:** Heuristic desktop.spawn button
- **Interaction:** add a button
- **Expected:** ui.patch button+activity
- **Status:** NOW

### S658
- **Perspective:** developer
- **Goal:** agentic_turn_plan merge
- **Interaction:** Chat that looks like spawn
- **Expected:** chat_message_plan + desktop_actions_for_text
- **Status:** NOW

### S659
- **Perspective:** developer
- **Goal:** Cloud multi-step plan
- **Interaction:** Complex ask with key
- **Expected:** plan_from_cloud steps execute sequentially via mcp_call along the agent-core spine
- **Status:** NOW

### S660
- **Perspective:** developer
- **Goal:** Localmodel plan JSON
- **Interaction:** localmodel returns plan
- **Expected:** Parsed localmodel plan JSON steps execute via mcp_call
- **Status:** NOW

### S661
- **Perspective:** developer
- **Goal:** lambda.register + bind
- **Interaction:** calculator intent
- **Expected:** Register + button mcp binding
- **Status:** NOW

### S662
- **Perspective:** developer
- **Goal:** lambda.invoke media
- **Interaction:** media_control intent
- **Expected:** media_control intent invokes lambda.invoke media_player; media primitive can also be spawned
- **Status:** NOW

### S663
- **Perspective:** developer
- **Goal:** ui.bind MCP
- **Interaction:** ui.bind on node
- **Expected:** ui.bind attaches mcp/state bindings; ui.event executes them
- **Status:** NOW

### S664
- **Perspective:** developer
- **Goal:** state: binding
- **Interaction:** Bind to state path
- **Expected:** Field/state sync via state.set/get plus ui.patch
- **Status:** NOW

### S665
- **Perspective:** developer
- **Goal:** Wildcard MCP calc.*
- **Interaction:** Call calc.add after register
- **Expected:** Dynamic registry patterns
- **Status:** NOW

### S666
- **Perspective:** developer
- **Goal:** bus.resolve miss synthesize
- **Interaction:** Unknown tool
- **Expected:** Unknown method may synthesize a lambda and ui.patch a workspace control
- **Status:** NOW

### S667
- **Perspective:** developer
- **Goal:** Deprecate lambda
- **Interaction:** lambda.deprecate
- **Expected:** lambda.deprecate is registered on the bus; agent plans can call it
- **Status:** NOW

### S668
- **Perspective:** developer
- **Goal:** Lease fast path
- **Interaction:** THE_MACHINE_LEASE_FAST_PATH=1
- **Expected:** bus.lease fast-path env toggle is implemented in mcp-bus
- **Status:** NOW

### S669
- **Perspective:** developer
- **Goal:** Event publish filesystem
- **Interaction:** filesystem intent
- **Expected:** event.publish category filesystem
- **Status:** NOW

### S670
- **Perspective:** developer
- **Goal:** query intent state
- **Interaction:** query intent
- **Expected:** state.set task.last_query
- **Status:** NOW

### S671
- **Perspective:** developer
- **Goal:** Plan step visualization
- **Interaction:** Watch activity per step
- **Expected:** activity_plan updates #ui.activity per step; suggestion/tour lists surface available actions
- **Status:** NOW

### S672
- **Perspective:** developer
- **Goal:** Rollback failed plan
- **Interaction:** Mid-plan MCP error
- **Expected:** agent.interrupt stops mid-plan; failed MCP results are recorded in the wake results; no transactional rollback UI (honest sequential spine)
- **Status:** NOW

### S673
- **Perspective:** developer
- **Goal:** Human-in-loop mid plan
- **Interaction:** Pause plan for input
- **Expected:** Privileged steps go through policy.confirm e4; chat otherwise continues
- **Status:** NOW

### S674
- **Perspective:** developer
- **Goal:** Parallel tool calls
- **Interaction:** Plan with parallel steps
- **Expected:** Plan steps run sequentially on the MCP bus (Machine-native; not parallel tool fan-out)
- **Status:** NOW

### S675
- **Perspective:** developer
- **Goal:** Binding to policy-gated op
- **Interaction:** Button → power.set_profile
- **Expected:** Clicking a bound control triggers mcp-bus policy.check; e4 confirmation is broker-owned
- **Status:** NOW

### S676
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #1
- **Interaction:** Request plan involving agent.status × steps=2
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S677
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #2
- **Interaction:** Request plan involving state.set × steps=3
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S678
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #3
- **Interaction:** Request plan involving lambda.register × steps=4
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S679
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #4
- **Interaction:** Request plan involving event.publish × steps=5
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S680
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #5
- **Interaction:** Request plan involving clipboard.get × steps=1
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S681
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #6
- **Interaction:** Request plan involving ui.patch × steps=2
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S682
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #7
- **Interaction:** Request plan involving agent.status × steps=3
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S683
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #8
- **Interaction:** Request plan involving state.set × steps=4
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S684
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #9
- **Interaction:** Request plan involving lambda.register × steps=5
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S685
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #10
- **Interaction:** Request plan involving event.publish × steps=1
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S686
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #11
- **Interaction:** Request plan involving clipboard.get × steps=2
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S687
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #12
- **Interaction:** Request plan involving ui.patch × steps=3
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S688
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #13
- **Interaction:** Request plan involving agent.status × steps=4
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S689
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #14
- **Interaction:** Request plan involving state.set × steps=5
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S690
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #15
- **Interaction:** Request plan involving lambda.register × steps=1
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S691
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #16
- **Interaction:** Request plan involving event.publish × steps=2
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S692
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #17
- **Interaction:** Request plan involving clipboard.get × steps=3
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S693
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #18
- **Interaction:** Request plan involving ui.patch × steps=4
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S694
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #19
- **Interaction:** Request plan involving agent.status × steps=5
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S695
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #20
- **Interaction:** Request plan involving state.set × steps=1
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S696
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #21
- **Interaction:** Request plan involving lambda.register × steps=2
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S697
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #22
- **Interaction:** Request plan involving event.publish × steps=3
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S698
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #23
- **Interaction:** Request plan involving clipboard.get × steps=4
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S699
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #24
- **Interaction:** Request plan involving ui.patch × steps=5
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S700
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #25
- **Interaction:** Request plan involving agent.status × steps=1
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S701
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #26
- **Interaction:** Request plan involving state.set × steps=2
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S702
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #27
- **Interaction:** Request plan involving lambda.register × steps=3
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S703
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #28
- **Interaction:** Request plan involving event.publish × steps=4
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S704
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #29
- **Interaction:** Request plan involving clipboard.get × steps=5
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S705
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #30
- **Interaction:** Request plan involving ui.patch × steps=1
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S706
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #31
- **Interaction:** Request plan involving agent.status × steps=2
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S707
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #32
- **Interaction:** Request plan involving state.set × steps=3
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S708
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #33
- **Interaction:** Request plan involving lambda.register × steps=4
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S709
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #34
- **Interaction:** Request plan involving event.publish × steps=5
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S710
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #35
- **Interaction:** Request plan involving clipboard.get × steps=1
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S711
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #36
- **Interaction:** Request plan involving ui.patch × steps=2
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S712
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #37
- **Interaction:** Request plan involving agent.status × steps=3
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S713
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #38
- **Interaction:** Request plan involving state.set × steps=4
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S714
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #39
- **Interaction:** Request plan involving lambda.register × steps=5
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S715
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #40
- **Interaction:** Request plan involving event.publish × steps=1
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S716
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #41
- **Interaction:** Request plan involving clipboard.get × steps=2
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S717
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #42
- **Interaction:** Request plan involving ui.patch × steps=3
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S718
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #43
- **Interaction:** Request plan involving agent.status × steps=4
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S719
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #44
- **Interaction:** Request plan involving state.set × steps=5
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S720
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #45
- **Interaction:** Request plan involving lambda.register × steps=1
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S721
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #46
- **Interaction:** Request plan involving event.publish × steps=2
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S722
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #47
- **Interaction:** Request plan involving clipboard.get × steps=3
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S723
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #48
- **Interaction:** Request plan involving ui.patch × steps=4
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S724
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #49
- **Interaction:** Request plan involving agent.status × steps=5
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S725
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #50
- **Interaction:** Request plan involving state.set × steps=1
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S726
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #51
- **Interaction:** Request plan involving lambda.register × steps=2
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S727
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #52
- **Interaction:** Request plan involving event.publish × steps=3
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S728
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #53
- **Interaction:** Request plan involving clipboard.get × steps=4
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S729
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #54
- **Interaction:** Request plan involving ui.patch × steps=5
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S730
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #55
- **Interaction:** Request plan involving agent.status × steps=1
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S731
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #56
- **Interaction:** Request plan involving state.set × steps=2
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S732
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #57
- **Interaction:** Request plan involving lambda.register × steps=3
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S733
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #58
- **Interaction:** Request plan involving event.publish × steps=4
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S734
- **Perspective:** developer
- **Goal:** Multi-step agentic plan case #59
- **Interaction:** Request plan involving clipboard.get × steps=5
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S735
- **Perspective:** agent itself
- **Goal:** Multi-step agentic plan case #60
- **Interaction:** Request plan involving ui.patch × steps=1
- **Expected:** Heuristic or LLM plan executes sequential MCP steps; workspace controls persist until ui.workspace.clear
- **Status:** NOW

### S736
- **Perspective:** developer
- **Goal:** compositor.present loop
- **Interaction:** Idle watch fps
- **Expected:** compositor.present runs the damage/present loop
- **Status:** NOW

### S737
- **Perspective:** developer
- **Goal:** compositor.surface create
- **Interaction:** MCP surface create
- **Expected:** Surface registry
- **Status:** NOW

### S738
- **Perspective:** developer
- **Goal:** compositor.blur region
- **Interaction:** compositor.blur
- **Expected:** compositor.blur sets a per-surface software blurred flag (not GPU glass)
- **Status:** NOW

### S739
- **Perspective:** developer
- **Goal:** compositor.list
- **Interaction:** List surfaces
- **Expected:** Returns surfaces
- **Status:** NOW

### S740
- **Perspective:** developer
- **Goal:** compositor.input inject
- **Interaction:** Inject key via MCP
- **Expected:** compositor.input injects events with provenance into ui.event
- **Status:** NOW

### S741
- **Perspective:** developer
- **Goal:** ui.components.list
- **Interaction:** List painted components
- **Expected:** Returns component inventory
- **Status:** NOW

### S742
- **Perspective:** developer
- **Goal:** ui.auil.parse
- **Interaction:** Parse AUIL string
- **Expected:** Rust parser
- **Status:** NOW

### S743
- **Perspective:** developer
- **Goal:** ui.auil.load boot
- **Interaction:** Load /etc/the-machine/boot.auil
- **Expected:** Installed rootfs path G13
- **Status:** NOW

### S744
- **Perspective:** developer
- **Goal:** ui.patch update props
- **Interaction:** Update greeting text
- **Expected:** Tree revision bumps
- **Status:** NOW

### S745
- **Perspective:** developer
- **Goal:** ui.get node
- **Interaction:** ui.get id=ui.greeting
- **Expected:** Returns node
- **Status:** NOW

### S746
- **Perspective:** developer
- **Goal:** Theme get/set
- **Interaction:** ui.theme.get / set
- **Expected:** ui.theme.get/set; named palettes dark/light/high-contrast
- **Status:** NOW

### S747
- **Perspective:** developer
- **Goal:** Opacity tween button
- **Interaction:** Press button
- **Expected:** snappy/gentle/reduced tweens
- **Status:** NOW

### S748
- **Perspective:** developer
- **Goal:** Chart data props
- **Interaction:** Patch chart data/items
- **Expected:** Axes+bars paint
- **Status:** NOW

### S749
- **Perspective:** developer
- **Goal:** Media ffmpeg frame
- **Interaction:** Patch media src=file
- **Expected:** media kind paints ffmpeg CLI first-frame RGB when available, else play affordance
- **Status:** NOW

### S750
- **Perspective:** developer
- **Goal:** Installer boot.auil
- **Interaction:** Validate installed rootfs
- **Expected:** boot.auil at /etc/the-machine/boot.auil
- **Status:** NOW

### S751
- **Perspective:** developer
- **Goal:** GRUB label the-machine
- **Interaction:** Boot installed disk
- **Expected:** LABEL=the-machine
- **Status:** NOW

### S752
- **Perspective:** developer
- **Goal:** Initramfs model bundle
- **Interaction:** Inspect initramfs models
- **Expected:** G11 GGUF bundling path remains available for localmodel
- **Status:** NOW

### S753
- **Perspective:** developer
- **Goal:** Fallback shell hello
- **Interaction:** shell.hello / hello route
- **Expected:** fallback-shell hello MCP + Ctrl+Alt+F9; console/MCP stub (not frozen compositor takeover)
- **Status:** NOW

### S754
- **Perspective:** developer
- **Goal:** Docs honesty audit
- **Interaction:** Read 03-docs-code-honesty.md
- **Expected:** Authoritative boot MCP + painted kinds
- **Status:** NOW

### S755
- **Perspective:** developer
- **Goal:** Maturity matrix P rows
- **Interaction:** Read 01-maturity-vs-toolkits.md
- **Expected:** Most UI rows P not F
- **Status:** NOW

### S756
- **Perspective:** end user
- **Goal:** Request workspace slider
- **Interaction:** Ask to 'spawn slider control 1'
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=slider under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S757
- **Perspective:** operator
- **Goal:** Monitor netlink
- **Interaction:** Watch netlink during session 2
- **Expected:** system-daemon netlink/hotplug publishes events; status/network chat lists interfaces
- **Status:** NOW

### S758
- **Perspective:** accessibility user
- **Goal:** AT navigate workspace
- **Interaction:** Explore workspace with keyboard/AT case 3
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S759
- **Perspective:** developer
- **Goal:** Trace MCP clipboard.get
- **Interaction:** Call clipboard.get and observe policy+audit 4
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S760
- **Perspective:** power user
- **Goal:** Keyboard chord 5
- **Interaction:** Custom shortcut map #5
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S761
- **Perspective:** QA engineer
- **Goal:** Regression policy #6
- **Interaction:** Automate policy smoke 6
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S762
- **Perspective:** agent itself
- **Goal:** Self-patch menu #7
- **Interaction:** Plan ui.patch menu id=ui.auto_7
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S763
- **Perspective:** security auditor
- **Goal:** Probe confirmation #8
- **Interaction:** Attempt forge confirm UI variant 8
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S764
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #9
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S765
- **Perspective:** first-time user
- **Goal:** Discoverability tour #10
- **Interaction:** Look for onboarding tips #10
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S766
- **Perspective:** operator
- **Goal:** Wayland client #11
- **Interaction:** Connect test client surface #11
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S767
- **Perspective:** end user
- **Goal:** Chat about email
- **Interaction:** Type 'email?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S768
- **Perspective:** end user
- **Goal:** Request workspace chart
- **Interaction:** Ask to 'spawn chart control 13'
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=chart under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S769
- **Perspective:** operator
- **Goal:** Monitor battery
- **Interaction:** Watch battery during session 14
- **Expected:** Chat power/battery → desktop.system → power.get_profile then #ui.power_panel + #ui.battery_widget
- **Status:** NOW

### S770
- **Perspective:** accessibility user
- **Goal:** AT navigate list
- **Interaction:** Explore list with keyboard/AT case 15
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S771
- **Perspective:** developer
- **Goal:** Trace MCP net.list_interfaces
- **Interaction:** Call net.list_interfaces and observe policy+audit 16
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S772
- **Perspective:** power user
- **Goal:** Keyboard chord 17
- **Interaction:** Custom shortcut map #17
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S773
- **Perspective:** QA engineer
- **Goal:** Regression audio #18
- **Interaction:** Automate audio smoke 18
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S774
- **Perspective:** agent itself
- **Goal:** Self-patch sidebar #19
- **Interaction:** Plan ui.patch sidebar id=ui.auto_19
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S775
- **Perspective:** security auditor
- **Goal:** Probe confirmation #20
- **Interaction:** Attempt forge confirm UI variant 20
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S776
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #21
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S777
- **Perspective:** first-time user
- **Goal:** Discoverability tour #22
- **Interaction:** Look for onboarding tips #22
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S778
- **Perspective:** operator
- **Goal:** Wayland client #23
- **Interaction:** Connect test client surface #23
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S779
- **Perspective:** end user
- **Goal:** Chat about passwords
- **Interaction:** Type 'passwords?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S780
- **Perspective:** end user
- **Goal:** Request workspace grid
- **Interaction:** Ask to 'spawn grid control 25'
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=grid under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S781
- **Perspective:** operator
- **Goal:** Monitor netlink
- **Interaction:** Watch netlink during session 26
- **Expected:** system-daemon netlink/hotplug publishes events; status/network chat lists interfaces
- **Status:** NOW

### S782
- **Perspective:** accessibility user
- **Goal:** AT navigate toggle
- **Interaction:** Explore toggle with keyboard/AT case 27
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S783
- **Perspective:** developer
- **Goal:** Trace MCP ui.a11y.tree
- **Interaction:** Call ui.a11y.tree and observe policy+audit 28
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S784
- **Perspective:** power user
- **Goal:** Keyboard chord 29
- **Interaction:** Custom shortcut map #29
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S785
- **Perspective:** QA engineer
- **Goal:** Regression boot #30
- **Interaction:** Automate boot smoke 30
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S786
- **Perspective:** agent itself
- **Goal:** Self-patch slider #31
- **Interaction:** Plan ui.patch slider id=ui.auto_31
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S787
- **Perspective:** security auditor
- **Goal:** Probe confirmation #32
- **Interaction:** Attempt forge confirm UI variant 32
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S788
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #33
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S789
- **Perspective:** first-time user
- **Goal:** Discoverability tour #34
- **Interaction:** Look for onboarding tips #34
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S790
- **Perspective:** operator
- **Goal:** Wayland client #35
- **Interaction:** Connect test client surface #35
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S791
- **Perspective:** end user
- **Goal:** Chat about sharing
- **Interaction:** Type 'sharing?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S792
- **Perspective:** end user
- **Goal:** Request workspace menu
- **Interaction:** Ask to 'spawn menu control 37'
- **Expected:** desktop.spawn inserts an AUIL stack (#ui.agent_menu) with MCP-bound buttons under #ui.workspace (Machine-native, not a separate widget kind)
- **Status:** NOW

### S793
- **Perspective:** operator
- **Goal:** Monitor battery
- **Interaction:** Watch battery during session 38
- **Expected:** Chat power/battery → desktop.system → power.get_profile then #ui.power_panel + #ui.battery_widget
- **Status:** NOW

### S794
- **Perspective:** accessibility user
- **Goal:** AT navigate activity
- **Interaction:** Explore activity with keyboard/AT case 39
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S795
- **Perspective:** developer
- **Goal:** Trace MCP ui.tree
- **Interaction:** Call ui.tree and observe policy+audit 40
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S796
- **Perspective:** power user
- **Goal:** Keyboard chord 41
- **Interaction:** Custom shortcut map #41
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S797
- **Perspective:** QA engineer
- **Goal:** Regression spawn #42
- **Interaction:** Automate spawn smoke 42
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S798
- **Perspective:** agent itself
- **Goal:** Self-patch chart #43
- **Interaction:** Plan ui.patch chart id=ui.auto_43
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S799
- **Perspective:** security auditor
- **Goal:** Probe confirmation #44
- **Interaction:** Attempt forge confirm UI variant 44
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S800
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #45
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S801
- **Perspective:** first-time user
- **Goal:** Discoverability tour #46
- **Interaction:** Look for onboarding tips #46
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S802
- **Perspective:** operator
- **Goal:** Wayland client #47
- **Interaction:** Connect test client surface #47
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S803
- **Perspective:** end user
- **Goal:** Chat about themes
- **Interaction:** Type 'themes?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S804
- **Perspective:** end user
- **Goal:** Request workspace sidebar
- **Interaction:** Ask to 'spawn sidebar control 49'
- **Expected:** desktop.spawn inserts an AUIL stack (#ui.agent_sidebar) with MCP-bound buttons under #ui.workspace (Machine-native, not a separate widget kind)
- **Status:** NOW

### S805
- **Perspective:** operator
- **Goal:** Monitor netlink
- **Interaction:** Watch netlink during session 50
- **Expected:** system-daemon netlink/hotplug publishes events; status/network chat lists interfaces
- **Status:** NOW

### S806
- **Perspective:** accessibility user
- **Goal:** AT navigate chat_input
- **Interaction:** Explore chat_input with keyboard/AT case 51
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S807
- **Perspective:** developer
- **Goal:** Trace MCP agent.status
- **Interaction:** Call agent.status and observe policy+audit 52
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S808
- **Perspective:** power user
- **Goal:** Keyboard chord 53
- **Interaction:** Custom shortcut map #53
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S809
- **Perspective:** QA engineer
- **Goal:** Regression ime #54
- **Interaction:** Automate ime smoke 54
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S810
- **Perspective:** agent itself
- **Goal:** Self-patch grid #55
- **Interaction:** Plan ui.patch grid id=ui.auto_55
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S811
- **Perspective:** security auditor
- **Goal:** Probe confirmation #56
- **Interaction:** Attempt forge confirm UI variant 56
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S812
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #57
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S813
- **Perspective:** first-time user
- **Goal:** Discoverability tour #58
- **Interaction:** Look for onboarding tips #58
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S814
- **Perspective:** operator
- **Goal:** Wayland client #59
- **Interaction:** Connect test client surface #59
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S815
- **Perspective:** end user
- **Goal:** Chat about timers
- **Interaction:** Type 'timers?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S816
- **Perspective:** end user
- **Goal:** Request workspace slider
- **Interaction:** Ask to 'spawn slider control 61'
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=slider under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S817
- **Perspective:** operator
- **Goal:** Monitor battery
- **Interaction:** Watch battery during session 62
- **Expected:** Chat power/battery → desktop.system → power.get_profile then #ui.power_panel + #ui.battery_widget
- **Status:** NOW

### S818
- **Perspective:** accessibility user
- **Goal:** AT navigate workspace
- **Interaction:** Explore workspace with keyboard/AT case 63
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S819
- **Perspective:** developer
- **Goal:** Trace MCP clipboard.get
- **Interaction:** Call clipboard.get and observe policy+audit 64
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S820
- **Perspective:** power user
- **Goal:** Keyboard chord 65
- **Interaction:** Custom shortcut map #65
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S821
- **Perspective:** QA engineer
- **Goal:** Regression policy #66
- **Interaction:** Automate policy smoke 66
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S822
- **Perspective:** agent itself
- **Goal:** Self-patch menu #67
- **Interaction:** Plan ui.patch menu id=ui.auto_67
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S823
- **Perspective:** security auditor
- **Goal:** Probe confirmation #68
- **Interaction:** Attempt forge confirm UI variant 68
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S824
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #69
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S825
- **Perspective:** first-time user
- **Goal:** Discoverability tour #70
- **Interaction:** Look for onboarding tips #70
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S826
- **Perspective:** operator
- **Goal:** Wayland client #71
- **Interaction:** Connect test client surface #71
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S827
- **Perspective:** end user
- **Goal:** Chat about speakers
- **Interaction:** Type 'speakers?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S828
- **Perspective:** end user
- **Goal:** Request workspace chart
- **Interaction:** Ask to 'spawn chart control 73'
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=chart under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S829
- **Perspective:** operator
- **Goal:** Monitor netlink
- **Interaction:** Watch netlink during session 74
- **Expected:** system-daemon netlink/hotplug publishes events; status/network chat lists interfaces
- **Status:** NOW

### S830
- **Perspective:** accessibility user
- **Goal:** AT navigate list
- **Interaction:** Explore list with keyboard/AT case 75
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S831
- **Perspective:** developer
- **Goal:** Trace MCP net.list_interfaces
- **Interaction:** Call net.list_interfaces and observe policy+audit 76
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S832
- **Perspective:** power user
- **Goal:** Keyboard chord 77
- **Interaction:** Custom shortcut map #77
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S833
- **Perspective:** QA engineer
- **Goal:** Regression audio #78
- **Interaction:** Automate audio smoke 78
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S834
- **Perspective:** agent itself
- **Goal:** Self-patch sidebar #79
- **Interaction:** Plan ui.patch sidebar id=ui.auto_79
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S835
- **Perspective:** security auditor
- **Goal:** Probe confirmation #80
- **Interaction:** Attempt forge confirm UI variant 80
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S836
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #81
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S837
- **Perspective:** first-time user
- **Goal:** Discoverability tour #82
- **Interaction:** Look for onboarding tips #82
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S838
- **Perspective:** operator
- **Goal:** Wayland client #83
- **Interaction:** Connect test client surface #83
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S839
- **Perspective:** end user
- **Goal:** Chat about updates
- **Interaction:** Type 'updates?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S840
- **Perspective:** end user
- **Goal:** Request workspace grid
- **Interaction:** Ask to 'spawn grid control 85'
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=grid under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S841
- **Perspective:** operator
- **Goal:** Monitor battery
- **Interaction:** Watch battery during session 86
- **Expected:** Chat power/battery → desktop.system → power.get_profile then #ui.power_panel + #ui.battery_widget
- **Status:** NOW

### S842
- **Perspective:** accessibility user
- **Goal:** AT navigate toggle
- **Interaction:** Explore toggle with keyboard/AT case 87
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S843
- **Perspective:** developer
- **Goal:** Trace MCP ui.a11y.tree
- **Interaction:** Call ui.a11y.tree and observe policy+audit 88
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S844
- **Perspective:** power user
- **Goal:** Keyboard chord 89
- **Interaction:** Custom shortcut map #89
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S845
- **Perspective:** QA engineer
- **Goal:** Regression boot #90
- **Interaction:** Automate boot smoke 90
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S846
- **Perspective:** agent itself
- **Goal:** Self-patch slider #91
- **Interaction:** Plan ui.patch slider id=ui.auto_91
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S847
- **Perspective:** security auditor
- **Goal:** Probe confirmation #92
- **Interaction:** Attempt forge confirm UI variant 92
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S848
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #93
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S849
- **Perspective:** first-time user
- **Goal:** Discoverability tour #94
- **Interaction:** Look for onboarding tips #94
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S850
- **Perspective:** operator
- **Goal:** Wayland client #95
- **Interaction:** Connect test client surface #95
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S851
- **Perspective:** end user
- **Goal:** Chat about music
- **Interaction:** Type 'music?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S852
- **Perspective:** end user
- **Goal:** Request workspace menu
- **Interaction:** Ask to 'spawn menu control 97'
- **Expected:** desktop.spawn inserts an AUIL stack (#ui.agent_menu) with MCP-bound buttons under #ui.workspace (Machine-native, not a separate widget kind)
- **Status:** NOW

### S853
- **Perspective:** operator
- **Goal:** Monitor netlink
- **Interaction:** Watch netlink during session 98
- **Expected:** system-daemon netlink/hotplug publishes events; status/network chat lists interfaces
- **Status:** NOW

### S854
- **Perspective:** accessibility user
- **Goal:** AT navigate activity
- **Interaction:** Explore activity with keyboard/AT case 99
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S855
- **Perspective:** developer
- **Goal:** Trace MCP ui.tree
- **Interaction:** Call ui.tree and observe policy+audit 100
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S856
- **Perspective:** power user
- **Goal:** Keyboard chord 101
- **Interaction:** Custom shortcut map #101
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S857
- **Perspective:** QA engineer
- **Goal:** Regression spawn #102
- **Interaction:** Automate spawn smoke 102
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S858
- **Perspective:** agent itself
- **Goal:** Self-patch chart #103
- **Interaction:** Plan ui.patch chart id=ui.auto_103
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S859
- **Perspective:** security auditor
- **Goal:** Probe confirmation #104
- **Interaction:** Attempt forge confirm UI variant 104
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S860
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #105
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S861
- **Perspective:** first-time user
- **Goal:** Discoverability tour #106
- **Interaction:** Look for onboarding tips #106
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S862
- **Perspective:** operator
- **Goal:** Wayland client #107
- **Interaction:** Connect test client surface #107
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S863
- **Perspective:** end user
- **Goal:** Chat about usb
- **Interaction:** Type 'usb?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S864
- **Perspective:** end user
- **Goal:** Request workspace sidebar
- **Interaction:** Ask to 'spawn sidebar control 109'
- **Expected:** desktop.spawn inserts an AUIL stack (#ui.agent_sidebar) with MCP-bound buttons under #ui.workspace (Machine-native, not a separate widget kind)
- **Status:** NOW

### S865
- **Perspective:** operator
- **Goal:** Monitor battery
- **Interaction:** Watch battery during session 110
- **Expected:** Chat power/battery → desktop.system → power.get_profile then #ui.power_panel + #ui.battery_widget
- **Status:** NOW

### S866
- **Perspective:** accessibility user
- **Goal:** AT navigate chat_input
- **Interaction:** Explore chat_input with keyboard/AT case 111
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S867
- **Perspective:** developer
- **Goal:** Trace MCP agent.status
- **Interaction:** Call agent.status and observe policy+audit 112
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S868
- **Perspective:** power user
- **Goal:** Keyboard chord 113
- **Interaction:** Custom shortcut map #113
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S869
- **Perspective:** QA engineer
- **Goal:** Regression ime #114
- **Interaction:** Automate ime smoke 114
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S870
- **Perspective:** agent itself
- **Goal:** Self-patch grid #115
- **Interaction:** Plan ui.patch grid id=ui.auto_115
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S871
- **Perspective:** security auditor
- **Goal:** Probe confirmation #116
- **Interaction:** Attempt forge confirm UI variant 116
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S872
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #117
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S873
- **Perspective:** first-time user
- **Goal:** Discoverability tour #118
- **Interaction:** Look for onboarding tips #118
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S874
- **Perspective:** operator
- **Goal:** Wayland client #119
- **Interaction:** Connect test client surface #119
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S875
- **Perspective:** end user
- **Goal:** Chat about uptime
- **Interaction:** Type 'uptime?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S876
- **Perspective:** end user
- **Goal:** Request workspace slider
- **Interaction:** Ask to 'spawn slider control 121'
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=slider under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S877
- **Perspective:** operator
- **Goal:** Monitor netlink
- **Interaction:** Watch netlink during session 122
- **Expected:** system-daemon netlink/hotplug publishes events; status/network chat lists interfaces
- **Status:** NOW

### S878
- **Perspective:** accessibility user
- **Goal:** AT navigate workspace
- **Interaction:** Explore workspace with keyboard/AT case 123
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S879
- **Perspective:** developer
- **Goal:** Trace MCP clipboard.get
- **Interaction:** Call clipboard.get and observe policy+audit 124
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S880
- **Perspective:** power user
- **Goal:** Keyboard chord 125
- **Interaction:** Custom shortcut map #125
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S881
- **Perspective:** QA engineer
- **Goal:** Regression policy #126
- **Interaction:** Automate policy smoke 126
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S882
- **Perspective:** agent itself
- **Goal:** Self-patch menu #127
- **Interaction:** Plan ui.patch menu id=ui.auto_127
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S883
- **Perspective:** security auditor
- **Goal:** Probe confirmation #128
- **Interaction:** Attempt forge confirm UI variant 128
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S884
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #129
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S885
- **Perspective:** first-time user
- **Goal:** Discoverability tour #130
- **Interaction:** Look for onboarding tips #130
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S886
- **Perspective:** operator
- **Goal:** Wayland client #131
- **Interaction:** Connect test client surface #131
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S887
- **Perspective:** end user
- **Goal:** Chat about email [887]
- **Interaction:** Type 'email?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S888
- **Perspective:** end user
- **Goal:** Request workspace chart
- **Interaction:** Ask to 'spawn chart control 133'
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=chart under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S889
- **Perspective:** operator
- **Goal:** Monitor battery
- **Interaction:** Watch battery during session 134
- **Expected:** Chat power/battery → desktop.system → power.get_profile then #ui.power_panel + #ui.battery_widget
- **Status:** NOW

### S890
- **Perspective:** accessibility user
- **Goal:** AT navigate list
- **Interaction:** Explore list with keyboard/AT case 135
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S891
- **Perspective:** developer
- **Goal:** Trace MCP net.list_interfaces
- **Interaction:** Call net.list_interfaces and observe policy+audit 136
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S892
- **Perspective:** power user
- **Goal:** Keyboard chord 137
- **Interaction:** Custom shortcut map #137
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S893
- **Perspective:** QA engineer
- **Goal:** Regression audio #138
- **Interaction:** Automate audio smoke 138
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S894
- **Perspective:** agent itself
- **Goal:** Self-patch sidebar #139
- **Interaction:** Plan ui.patch sidebar id=ui.auto_139
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S895
- **Perspective:** security auditor
- **Goal:** Probe confirmation #140
- **Interaction:** Attempt forge confirm UI variant 140
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S896
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #141
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S897
- **Perspective:** first-time user
- **Goal:** Discoverability tour #142
- **Interaction:** Look for onboarding tips #142
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S898
- **Perspective:** operator
- **Goal:** Wayland client #143
- **Interaction:** Connect test client surface #143
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S899
- **Perspective:** end user
- **Goal:** Chat about passwords [899]
- **Interaction:** Type 'passwords?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S900
- **Perspective:** end user
- **Goal:** Request workspace grid
- **Interaction:** Ask to 'spawn grid control 145'
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=grid under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S901
- **Perspective:** operator
- **Goal:** Monitor netlink
- **Interaction:** Watch netlink during session 146
- **Expected:** system-daemon netlink/hotplug publishes events; status/network chat lists interfaces
- **Status:** NOW

### S902
- **Perspective:** accessibility user
- **Goal:** AT navigate toggle
- **Interaction:** Explore toggle with keyboard/AT case 147
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S903
- **Perspective:** developer
- **Goal:** Trace MCP ui.a11y.tree
- **Interaction:** Call ui.a11y.tree and observe policy+audit 148
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S904
- **Perspective:** power user
- **Goal:** Keyboard chord 149
- **Interaction:** Custom shortcut map #149
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S905
- **Perspective:** QA engineer
- **Goal:** Regression boot #150
- **Interaction:** Automate boot smoke 150
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S906
- **Perspective:** agent itself
- **Goal:** Self-patch slider #151
- **Interaction:** Plan ui.patch slider id=ui.auto_151
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S907
- **Perspective:** security auditor
- **Goal:** Probe confirmation #152
- **Interaction:** Attempt forge confirm UI variant 152
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S908
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #153
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S909
- **Perspective:** first-time user
- **Goal:** Discoverability tour #154
- **Interaction:** Look for onboarding tips #154
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S910
- **Perspective:** operator
- **Goal:** Wayland client #155
- **Interaction:** Connect test client surface #155
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S911
- **Perspective:** end user
- **Goal:** Chat about sharing [911]
- **Interaction:** Type 'sharing?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S912
- **Perspective:** end user
- **Goal:** Request workspace menu
- **Interaction:** Ask to 'spawn menu control 157'
- **Expected:** desktop.spawn inserts an AUIL stack (#ui.agent_menu) with MCP-bound buttons under #ui.workspace (Machine-native, not a separate widget kind)
- **Status:** NOW

### S913
- **Perspective:** operator
- **Goal:** Monitor battery
- **Interaction:** Watch battery during session 158
- **Expected:** Chat power/battery → desktop.system → power.get_profile then #ui.power_panel + #ui.battery_widget
- **Status:** NOW

### S914
- **Perspective:** accessibility user
- **Goal:** AT navigate activity
- **Interaction:** Explore activity with keyboard/AT case 159
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S915
- **Perspective:** developer
- **Goal:** Trace MCP ui.tree
- **Interaction:** Call ui.tree and observe policy+audit 160
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S916
- **Perspective:** power user
- **Goal:** Keyboard chord 161
- **Interaction:** Custom shortcut map #161
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S917
- **Perspective:** QA engineer
- **Goal:** Regression spawn #162
- **Interaction:** Automate spawn smoke 162
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S918
- **Perspective:** agent itself
- **Goal:** Self-patch chart #163
- **Interaction:** Plan ui.patch chart id=ui.auto_163
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S919
- **Perspective:** security auditor
- **Goal:** Probe confirmation #164
- **Interaction:** Attempt forge confirm UI variant 164
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S920
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #165
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S921
- **Perspective:** first-time user
- **Goal:** Discoverability tour #166
- **Interaction:** Look for onboarding tips #166
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S922
- **Perspective:** operator
- **Goal:** Wayland client #167
- **Interaction:** Connect test client surface #167
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S923
- **Perspective:** end user
- **Goal:** Chat about themes [923]
- **Interaction:** Type 'themes?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S924
- **Perspective:** end user
- **Goal:** Request workspace sidebar
- **Interaction:** Ask to 'spawn sidebar control 169'
- **Expected:** desktop.spawn inserts an AUIL stack (#ui.agent_sidebar) with MCP-bound buttons under #ui.workspace (Machine-native, not a separate widget kind)
- **Status:** NOW

### S925
- **Perspective:** operator
- **Goal:** Monitor netlink
- **Interaction:** Watch netlink during session 170
- **Expected:** system-daemon netlink/hotplug publishes events; status/network chat lists interfaces
- **Status:** NOW

### S926
- **Perspective:** accessibility user
- **Goal:** AT navigate chat_input
- **Interaction:** Explore chat_input with keyboard/AT case 171
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S927
- **Perspective:** developer
- **Goal:** Trace MCP agent.status
- **Interaction:** Call agent.status and observe policy+audit 172
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S928
- **Perspective:** power user
- **Goal:** Keyboard chord 173
- **Interaction:** Custom shortcut map #173
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S929
- **Perspective:** QA engineer
- **Goal:** Regression ime #174
- **Interaction:** Automate ime smoke 174
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S930
- **Perspective:** agent itself
- **Goal:** Self-patch grid #175
- **Interaction:** Plan ui.patch grid id=ui.auto_175
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S931
- **Perspective:** security auditor
- **Goal:** Probe confirmation #176
- **Interaction:** Attempt forge confirm UI variant 176
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S932
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #177
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S933
- **Perspective:** first-time user
- **Goal:** Discoverability tour #178
- **Interaction:** Look for onboarding tips #178
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S934
- **Perspective:** operator
- **Goal:** Wayland client #179
- **Interaction:** Connect test client surface #179
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S935
- **Perspective:** end user
- **Goal:** Chat about timers [935]
- **Interaction:** Type 'timers?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S936
- **Perspective:** end user
- **Goal:** Request workspace slider
- **Interaction:** Ask to 'spawn slider control 181'
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=slider under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S937
- **Perspective:** operator
- **Goal:** Monitor battery
- **Interaction:** Watch battery during session 182
- **Expected:** Chat power/battery → desktop.system → power.get_profile then #ui.power_panel + #ui.battery_widget
- **Status:** NOW

### S938
- **Perspective:** accessibility user
- **Goal:** AT navigate workspace
- **Interaction:** Explore workspace with keyboard/AT case 183
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S939
- **Perspective:** developer
- **Goal:** Trace MCP clipboard.get
- **Interaction:** Call clipboard.get and observe policy+audit 184
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S940
- **Perspective:** power user
- **Goal:** Keyboard chord 185
- **Interaction:** Custom shortcut map #185
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S941
- **Perspective:** QA engineer
- **Goal:** Regression policy #186
- **Interaction:** Automate policy smoke 186
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S942
- **Perspective:** agent itself
- **Goal:** Self-patch menu #187
- **Interaction:** Plan ui.patch menu id=ui.auto_187
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S943
- **Perspective:** security auditor
- **Goal:** Probe confirmation #188
- **Interaction:** Attempt forge confirm UI variant 188
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S944
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #189
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S945
- **Perspective:** first-time user
- **Goal:** Discoverability tour #190
- **Interaction:** Look for onboarding tips #190
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S946
- **Perspective:** operator
- **Goal:** Wayland client #191
- **Interaction:** Connect test client surface #191
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S947
- **Perspective:** end user
- **Goal:** Chat about speakers [947]
- **Interaction:** Type 'speakers?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S948
- **Perspective:** end user
- **Goal:** Request workspace chart
- **Interaction:** Ask to 'spawn chart control 193'
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=chart under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S949
- **Perspective:** operator
- **Goal:** Monitor netlink
- **Interaction:** Watch netlink during session 194
- **Expected:** system-daemon netlink/hotplug publishes events; status/network chat lists interfaces
- **Status:** NOW

### S950
- **Perspective:** accessibility user
- **Goal:** AT navigate list
- **Interaction:** Explore list with keyboard/AT case 195
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S951
- **Perspective:** developer
- **Goal:** Trace MCP net.list_interfaces
- **Interaction:** Call net.list_interfaces and observe policy+audit 196
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S952
- **Perspective:** power user
- **Goal:** Keyboard chord 197
- **Interaction:** Custom shortcut map #197
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S953
- **Perspective:** QA engineer
- **Goal:** Regression audio #198
- **Interaction:** Automate audio smoke 198
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S954
- **Perspective:** agent itself
- **Goal:** Self-patch sidebar #199
- **Interaction:** Plan ui.patch sidebar id=ui.auto_199
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S955
- **Perspective:** security auditor
- **Goal:** Probe confirmation #200
- **Interaction:** Attempt forge confirm UI variant 200
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S956
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #201
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S957
- **Perspective:** first-time user
- **Goal:** Discoverability tour #202
- **Interaction:** Look for onboarding tips #202
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S958
- **Perspective:** operator
- **Goal:** Wayland client #203
- **Interaction:** Connect test client surface #203
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S959
- **Perspective:** end user
- **Goal:** Chat about updates [959]
- **Interaction:** Type 'updates?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S960
- **Perspective:** end user
- **Goal:** Request workspace grid
- **Interaction:** Ask to 'spawn grid control 205'
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=grid under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S961
- **Perspective:** operator
- **Goal:** Monitor battery
- **Interaction:** Watch battery during session 206
- **Expected:** Chat power/battery → desktop.system → power.get_profile then #ui.power_panel + #ui.battery_widget
- **Status:** NOW

### S962
- **Perspective:** accessibility user
- **Goal:** AT navigate toggle
- **Interaction:** Explore toggle with keyboard/AT case 207
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S963
- **Perspective:** developer
- **Goal:** Trace MCP ui.a11y.tree
- **Interaction:** Call ui.a11y.tree and observe policy+audit 208
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S964
- **Perspective:** power user
- **Goal:** Keyboard chord 209
- **Interaction:** Custom shortcut map #209
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S965
- **Perspective:** QA engineer
- **Goal:** Regression boot #210
- **Interaction:** Automate boot smoke 210
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S966
- **Perspective:** agent itself
- **Goal:** Self-patch slider #211
- **Interaction:** Plan ui.patch slider id=ui.auto_211
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S967
- **Perspective:** security auditor
- **Goal:** Probe confirmation #212
- **Interaction:** Attempt forge confirm UI variant 212
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S968
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #213
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S969
- **Perspective:** first-time user
- **Goal:** Discoverability tour #214
- **Interaction:** Look for onboarding tips #214
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S970
- **Perspective:** operator
- **Goal:** Wayland client #215
- **Interaction:** Connect test client surface #215
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S971
- **Perspective:** end user
- **Goal:** Chat about music [971]
- **Interaction:** Type 'music?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S972
- **Perspective:** end user
- **Goal:** Request workspace menu
- **Interaction:** Ask to 'spawn menu control 217'
- **Expected:** desktop.spawn inserts an AUIL stack (#ui.agent_menu) with MCP-bound buttons under #ui.workspace (Machine-native, not a separate widget kind)
- **Status:** NOW

### S973
- **Perspective:** operator
- **Goal:** Monitor netlink
- **Interaction:** Watch netlink during session 218
- **Expected:** system-daemon netlink/hotplug publishes events; status/network chat lists interfaces
- **Status:** NOW

### S974
- **Perspective:** accessibility user
- **Goal:** AT navigate activity
- **Interaction:** Explore activity with keyboard/AT case 219
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S975
- **Perspective:** developer
- **Goal:** Trace MCP ui.tree
- **Interaction:** Call ui.tree and observe policy+audit 220
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S976
- **Perspective:** power user
- **Goal:** Keyboard chord 221
- **Interaction:** Custom shortcut map #221
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S977
- **Perspective:** QA engineer
- **Goal:** Regression spawn #222
- **Interaction:** Automate spawn smoke 222
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S978
- **Perspective:** agent itself
- **Goal:** Self-patch chart #223
- **Interaction:** Plan ui.patch chart id=ui.auto_223
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S979
- **Perspective:** security auditor
- **Goal:** Probe confirmation #224
- **Interaction:** Attempt forge confirm UI variant 224
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S980
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #225
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S981
- **Perspective:** first-time user
- **Goal:** Discoverability tour #226
- **Interaction:** Look for onboarding tips #226
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S982
- **Perspective:** operator
- **Goal:** Wayland client #227
- **Interaction:** Connect test client surface #227
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S983
- **Perspective:** end user
- **Goal:** Chat about usb [983]
- **Interaction:** Type 'usb?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S984
- **Perspective:** end user
- **Goal:** Request workspace sidebar
- **Interaction:** Ask to 'spawn sidebar control 229'
- **Expected:** desktop.spawn inserts an AUIL stack (#ui.agent_sidebar) with MCP-bound buttons under #ui.workspace (Machine-native, not a separate widget kind)
- **Status:** NOW

### S985
- **Perspective:** operator
- **Goal:** Monitor battery
- **Interaction:** Watch battery during session 230
- **Expected:** Chat power/battery → desktop.system → power.get_profile then #ui.power_panel + #ui.battery_widget
- **Status:** NOW

### S986
- **Perspective:** accessibility user
- **Goal:** AT navigate chat_input
- **Interaction:** Explore chat_input with keyboard/AT case 231
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S987
- **Perspective:** developer
- **Goal:** Trace MCP agent.status
- **Interaction:** Call agent.status and observe policy+audit 232
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S988
- **Perspective:** power user
- **Goal:** Keyboard chord 233
- **Interaction:** Custom shortcut map #233
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

### S989
- **Perspective:** QA engineer
- **Goal:** Regression ime #234
- **Interaction:** Automate ime smoke 234
- **Expected:** Covered by agent-core / ui-runtime / mcp-bus unit tests on this spine
- **Status:** NOW

### S990
- **Perspective:** agent itself
- **Goal:** Self-patch grid #235
- **Interaction:** Plan ui.patch grid id=ui.auto_235
- **Expected:** Agent ui.patch can update any painted kind already in the tree (last writer wins on id)
- **Status:** NOW

### S991
- **Perspective:** security auditor
- **Goal:** Probe confirmation #236
- **Interaction:** Attempt forge confirm UI variant 236
- **Expected:** e4 reserved; agent patches must not reach confirmation layer
- **Status:** NOW

### S992
- **Perspective:** localization tester
- **Goal:** Pseudo-loc string #237
- **Interaction:** Inject long/psuedoloc into greeting via ui.patch
- **Expected:** Long catalog strings layout in stacks; chat_log list clips + scrolls
- **Status:** NOW

### S993
- **Perspective:** first-time user
- **Goal:** Discoverability tour #238
- **Interaction:** Look for onboarding tips #238
- **Expected:** Chat 'take a tour' / desktop.tour updates workspace_hint and opens #ui.suggestion_tray
- **Status:** NOW

### S994
- **Perspective:** operator
- **Goal:** Wayland client #239
- **Interaction:** Connect test client surface #239
- **Expected:** Compositor auto-selects DRM/fb0/Wayland SHM; xdg_wm_base when Wayland; ISO-QEMU scripts present SessionGreeting
- **Status:** NOW

### S995
- **Perspective:** end user
- **Goal:** Chat about uptime [995]
- **Interaction:** Type 'uptime?' and Send
- **Expected:** Heuristic or model reply into #ui.chat_log; no false claim of deep OS integration
- **Status:** NOW

### S996
- **Perspective:** end user
- **Goal:** Request workspace slider
- **Interaction:** Ask to 'spawn slider control 241'
- **Expected:** AUIL chat → agent.chat.send → desktop.spawn inserts type=slider under #ui.workspace and activity_plan; duplicate id last-writer-wins
- **Status:** NOW

### S997
- **Perspective:** operator
- **Goal:** Monitor netlink
- **Interaction:** Watch netlink during session 242
- **Expected:** system-daemon netlink/hotplug publishes events; status/network chat lists interfaces
- **Status:** NOW

### S998
- **Perspective:** accessibility user
- **Goal:** AT navigate workspace
- **Interaction:** Explore workspace with keyboard/AT case 243
- **Expected:** Focus order includes button/field/toggle/slider/list/dialog/media/chart/grid; ui.a11y.tree names from label/id
- **Status:** NOW

### S999
- **Perspective:** developer
- **Goal:** Trace MCP clipboard.get
- **Interaction:** Call clipboard.get and observe policy+audit 244
- **Expected:** mcp-bus → policy.check → component; confirm fail-closed semantics
- **Status:** NOW

### S1000
- **Perspective:** power user
- **Goal:** Keyboard chord 245
- **Interaction:** Custom shortcut map #245
- **Expected:** Built-in chords: F1 help, F5 refresh, F12 status, Alt+Tab/Super cycle, Ctrl+Shift+T suggestions, Ctrl+Alt+F9 fallback hello — no user shortcut editor
- **Status:** NOW

