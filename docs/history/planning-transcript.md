# Planning conversation transcript

Cleaned export of the Claude planning conversation that produced Croncave's product definition, architecture and pricing. User messages and Claude's replies only; tool calls and internal context are omitted. The live session: https://claude.ai/code/session_01RBHRWXhyB6ekKdP7Udki85 (private to the account owner).


---

## User

One of the issues I encounter often is that I want to offload some workflow to the cloud. it could be a scraper, an agentic coding session, a stock tracking algorithm, a job that pushes updates to some external system, etc. However it's hard to even know where to get started (and I'm a software engineer). I want to build a platform that allows people (including non-technical people) to do this seamlessly, in a way that they understand. Note that this is meant purely for computing, not for accepting requests - it is not a server. I'm aware platforms like this already exist, but none of them are easy to use for non technical people


---

## Claude

The gap you're describing is real. Zapier and n8n are easy but shaped around integrations: "when X happens in app A, do Y in app B." Modal, GitHub Actions, and Trigger.dev are shaped around compute, but they assume you know what a container, a cron expression, and an env var are. Few tools say "here's a thing I want running in the background, keep it alive and tell me what it's doing" in terms a non-engineer understands.

**The product is the mental model, not the compute**

Running code in the cloud is solved. What's missing is a vocabulary a non-technical person can reason with. I'd make every job answer five plain questions, and make the UI nothing more than those answers:

1. **When does it run?** On a schedule, when something changes (a page, a price, an inbox), manually, or "keep going until done" for agentic sessions.
2. **What does it do?** Described in English, with the generated code available but never required.
3. **What does it remember?** This is the most underrated piece. A stock tracker, a scraper that alerts on changes, or a sync job all need state between runs. Give every job a small, visible memory: "last price seen: $182.40."
4. **Where do results go?** Email, a spreadsheet, Slack, a push notification, an external API.
5. **What are the limits?** A hard cost ceiling, a max runtime, and what happens on failure.

With those five fields as the source of truth, an LLM can turn "check this site every morning and text me if the price drops under $50" into a job card the person can read and verify, even if they could never read the code behind it.

**Trust features matter more than power features**

Non-technical users will abandon the platform the first time a job silently breaks or runs up a bill. So:

- **Dry runs** that show "here's what would have been sent" before going live.
- **Run history as a plain-language timeline**: "Ran at 8:00, found 3 new listings, emailed you."
- **Proactive breakage detection.** Scrapers rot constantly, so "this site changed its layout, want me to fix the job?" could be a killer feature, since an agent can often repair it.
- **Spend caps that are impossible to miss.**

**Where Rust fits**

Rust is a strong fit for the platform itself, less so for the jobs users run.

- **Control plane and scheduler:** Tokio plus axum for the API, with Postgres for job definitions and run history. Reliable long-running schedulers are where Rust's lack of GC pauses and its correctness guarantees pay off.
- **Sandboxing untrusted code:** Both leading options are written in Rust.
  - **Firecracker** microVMs handle heavy jobs like Playwright scrapers and agentic coding sessions, which need a real Linux environment.
  - **Wasmtime** handles lightweight jobs. The Wasm component model gives you capability-based security, so a job can only reach the network hosts or secrets you've granted it. That maps neatly onto a non-technical permissions UI: "this job can access: example.com, your Gmail."
- **Durable execution:** Long agentic sessions need checkpointing so a crash resumes rather than restarts. You could build a small durable-workflow engine in Rust or look at existing ones (Restate's server is Rust).
- **Job code:** Realistically, generated jobs will mostly be Python or TypeScript, since that's where scraping libraries and LLM SDKs live and what an LLM writes most reliably. Rust runs the platform; users never see it.

**Risks to plan for early**

- **Abuse:** free compute attracts crypto miners and people scraping sites that don't want it. You'll need egress controls and a ToS stance from day one.
- **Cost of agentic jobs:** a coding session can burn far more in model tokens than in compute, so pricing needs to account for that.
- **Scope:** "any workflow" is too broad to build well at first.

That last one leads to my main question: who's the first user, and what's their first job? A strong wedge would be something like "monitor anything and alert me" (prices, listings, pages, stocks). It covers several of your examples, needs state and triggers but not heavy compute, and a non-technical person immediately gets it. Is that the kind of starting point you had in mind, or are you more drawn to the agentic-session side?


---

## User

Some of these capture my vision, but not so much. A good way to visualize it is "your computer, but on the cloud", or basically "using a remote machine". Essentially you can do things you would normally do on your computer, but on a remote machine, but without all the hassle it normally takes. Obviously, the use cases would be limited as well - you can't use iMessage or google photos on the platform. It's purely for workloads that you want to be hands-off from. Users can spin up as many of these environments as they want for different tasks. They can also assign multiple tasks to one "machine". It's up to them.
Let's explore a few things this platform can do:

1. Agentic sessions such as coding - interacting with claude code, openAI, etc to build something, but on the cloud. You bring your AI provider API key, you can close your laptop and go to sleep, and wake up and it's done. You can connect your GitHub, etc.
2. Scheduled sessions where you configure a schedule and it runs some sort of job. This may be a job you manually create and upload, or a job you ask AI to create. This includes scrapers, monitoring, etc. Note that these jobs may query external data.
3. "monitor anything and alert me" jobs. I assume these would use some sort of polling session.
4. Triggered sessions based on external integrations (stretch goal) - not sure how to implement this without enabling servers, maybe polling. or webhooks?. Since I want this to be secure and private, I don't want these environments to be accessible.

Note that while I want to avoid "servers", there is no restriction on querying external data (e.g "build me a scraper" would probably want to query external data).
Importantly, users should be able to set things up manually (but easily), or ask AI to set things up and then edit it. I want to follow apple's UX pattern where they expose easy and common operations, while allowing users to expand into more advanced details if they need to. For example, when selecting how powerful one wants a machine to be, we offer default options with minimal detail, e.g "Default - 16GB RAM, 120GB Storage - great for everyday tasks like...". However users can decide to learn more about the machine if they want.


---

## Claude

That reframing makes it much clearer, and I think it's a stronger product. "Your computer, but in the cloud" gives non-technical users a mental model they already have. They know what a computer is, that it has files, that you can leave something running on it. You don't have to teach them "jobs" or "workflows."

**The core idea: machines that sleep**

If the core noun is a **Machine** and the secondary noun is a **Task** that lives on it, the key design question is cost. People won't spin up five machines if each one costs as much as an always-on VPS. So I'd make machines **sleep by default, like a laptop with the lid closed**:

- The disk persists: files, installed tools, and the job's memory all stay.
- Compute only runs when a task needs it: a schedule fires, an agent session is active, or the user opens a terminal.
- Billing becomes "awake time + storage," which is easy to explain: "This machine was awake 3 hours this week."

Firecracker (written in Rust) fits this well. It can snapshot a running microVM and restore it quickly, so waking a machine feels instant rather than like a cold boot.

**The security principle: machines only dial out**

Your "not accessible, no servers" requirement can be one hard rule: **a machine never accepts inbound connections.** A small agent inside each machine, which I'd write in Rust as a single static binary, opens an outbound connection to your control plane. Everything flows through that channel:

- the browser terminal and file browser
- live logs and "attach to this session from my phone"
- task commands and incoming events

This is the same trick Tailscale and similar agents use. Users get full interactive access, yet the machine has no open ports and no public address.

**The four use cases**

**1. Agentic sessions.** The machine comes with agent CLIs preinstalled. The session runs in a persistent terminal (think tmux under the hood), so closing the laptop changes nothing, and the user can reattach from anywhere. A few details matter a lot:

- **Keep the API key off the machine.** Route the agent's API calls through a proxy on your control plane that injects the key. If an agent gets prompt-injected by a malicious README, there's no key on disk to steal. The same proxy lets you enforce a spend cap even with bring-your-own keys, and show "this session used $4.12 of tokens."
- **Use a GitHub App with per-repo access** rather than personal tokens, so "connect GitHub" is one button with a clear scope.
- **Autonomy becomes more reasonable.** Since the machine is an isolated sandbox, letting the agent skip permission prompts is much less dangerous than on a personal laptop, which is what makes "wake up and it's done" realistic. Still offer a "ping me if it needs a decision" mode with push notifications.

**2. Scheduled tasks.** The schedule lives in the control plane, not the machine. It wakes the machine, runs the command, captures output, and lets it sleep again. A task is just a folder, a command, a schedule, and a notification rule. The important principle is that **AI setup and manual setup produce the exact same thing.** The AI fills in the same form a person would, so "ask AI, then edit" has no separate code path, and nothing becomes magic that can't be inspected.

**3. Monitor and alert.** I don't think this needs its own primitive. It's a scheduled task plus stored state ("last value seen") plus a condition plus a notification: a template on top of #2 with a friendly form. One optimization for later: waking a VM every minute just to check a price is wasteful, so very lightweight checks could run as sandboxed Wasm on the control plane (Wasmtime, also Rust) without waking the machine.

**4. Triggers.** Webhooks work without breaking the security rule, because **the platform receives the webhook, never the machine**:

1. You issue a unique URL per trigger.
2. The platform verifies the signature (GitHub, Stripe, etc.) and queues the event.
3. It wakes the machine and hands the event to the task over the outbound channel.

The platform is the server, so the user's machines never have to be. For services without webhooks, such as an email inbox, the platform polls on the user's behalf. To the user, both look the same: "When a new email arrives from X…"

**Apple-style layering**

Your machine-size example works well. The same pattern applies everywhere:

| Layer | Machine | Task |
|---|---|---|
| **Default** | "Standard: great for coding sessions and scrapers" | "Every morning at 8, email me the results" |
| **Details** | 16GB RAM, 4 CPUs, 120GB disk, estimated cost per hour awake | Timeout, retry on failure, notification rules |
| **Expert** | Base image, installed packages, environment variables, a terminal | The actual script, cron expression, raw logs |

One wording choice matters: machines will run Linux, but I'd barely mention it. "It's a computer that runs your tasks" is enough at the top layer, with "Ubuntu 24.04" available in the details.

**Where Rust carries the platform**

- **Control plane and scheduler:** Tokio plus axum, with Postgres.
- **In-machine agent:** a Rust static binary handling the tunnel, task runner, log shipping, and file sync. It's tiny, has no runtime dependencies, and is easy to bake into every image.
- **Execution:** Firecracker for machines, and optionally Wasmtime for lightweight checks.

**Open design question: tasks sharing a machine**

When several tasks share a machine, what happens if two run at the same time? Can a nightly scraper interrupt a coding session, or should tasks queue? I'd treat the machine like a real computer: tasks run concurrently as separate processes, with a per-task "don't run while X is running" option in the details layer. It's worth deciding early, though, because it shapes the scheduler.

Would it help to go deeper on the architecture next, or to mock up the core screens (the machine list, "new machine," and "new task" flows) so you can see how the layering feels in practice?


---

## User

I think you're still not clear on what I want. First, I want this to be a web platform. Second, I don't want to offer users a desktop UI. The only UI they see is my own. We will need to come up with a novel UI/UX that allows people to manage their machines and work without the typical desktop environment. I'm thinking of two conceptual ways we could present this:

1. The idea of "machines". Machines can have multiple workloads/tasks assigned to them
2. Abstract machines away completely, use only the idea of workloads/tasks
3. A merger of the two - workspaces. Workspaces can have multiple tasks which may or may not be connected. However, workspaces don't have to be used - tasks can be created standalone.

I'm also open to more conceptual ideas. One thing that is common among all is that multiple machines, workspaces, tasks, etc could actually all run on the same VM, but just be presented separately, to save costs. Now let me give some example scenarios.
Founder
I am a founder of a real estate app. Almost daily, I vibe code updates to the app based on user feedback, etc. However, I do this on my laptop (with a coding agent of course), and I push the code to github, etc. I usually have to be fully present and make sure my laptop does not shut down. If I get sleepy, I have to stop there, I can’t delegate AI to keep working.
Enter our platform. Today I want to make some updates to my real estate app. I log into our platform and see a dashboard with all of my active workspaces, go to my “real estate app” workspace, and I tell AI I want to make certain changes. I go to sleep, wake up, and it’s done. Because I enabled auto-push, AI also pushed the changes to my connected github repo. End of scenario. For this scenario, we need to figure out how to present the code changes, etc to the user. Perhaps an in platform “IDE”? But that may be overkill. Maybe something else.
Finance bro
I am a finance bro and I want to build a system to monitor stock prices in a certain way and notify me of certain occurrences (or even trade on my behalf). I log into our platform, immediately start a new AI chat, and say what I want to do. I discuss with AI a bit and once I’m satisfied, AI spins up my task/workspace/machine, sets up everything, builds the monitoring system, configures my notifications (in-app, email, text, maybe a mobile app) and if auto-start is on, starts the system.
Prototyper
I am a designer who likes to build prototypes for work presentations. I log on to the platform, ask AI to prototype one of my ideas, come back, and I have results. Aside: We also need to figure out a way for them to interact with these. Should we offer an in-platform “browser” interface?

Data cruncher
I need to crunch through a large csv file and extract data into a certain format, then make batch queries to some external API with that data. I ask the platform, and it builds me an entire system, as well as an interface for interacting with my system.


These are just some use cases. There could be many more


---

## Claude

Got it. The picture is now: a web app where the product's own UI replaces the desktop entirely, and the compute underneath is an implementation detail. That changes the design problem from "how do we make a remote computer easy" to "what does a person need to see and do, given that an AI is doing the work on hidden compute?"

**What your four scenarios have in common**

None of these users actually cares about the machine. Each one cares about a *thing that exists because of the work*:

- The founder has a **codebase that changed**.
- The finance bro has a **system that's running** and watching for something.
- The prototyper has a **prototype to look at and show people**.
- The data cruncher has **results, plus a tool to run it again**.

So I'd drop machines from the primary UI entirely. The word "machine" invites the question "what's on it?", and that question drags you back toward a desktop. Your cost-saving plan of packing many workspaces onto shared VMs also becomes invisible and uncontroversial. Machines can still appear in the details layer as "resources: Standard, 16GB" for the people who care. One firm rule underneath: never pack different users onto the same VM, and isolate a single user's workspaces from each other with containers, so a runaway scraper can't take down an overnight coding session.

**The model I'd pick: your option 3, simplified**

Workspaces with optional standalone tasks is the right instinct. The risk is having two parallel structures, where users wonder "is this a task or a workspace?" and you maintain two code paths. I'd make it one structure underneath: **every task lives in a workspace, but a standalone task gets an implicit, invisible one.** It looks standalone in the UI, and the moment the user adds a second related task, it quietly becomes a named workspace. It's the same trick as Apple Notes, where folders exist but you never have to think about them.

**The novel part: views instead of a desktop**

A desktop gives you a generic interface for everything. Your platform can do something a desktop can't: **the AI builds a purpose-made interface for each workspace.** I'd call these **Views**:

- The finance bro gets a live dashboard: watched tickers, current values against thresholds, alert history, and a pause button.
- The data cruncher gets a small app: upload a CSV, see progress through the batch, preview results, download the output, rerun with a new file.
- The prototyper's view *is* the prototype.
- The founder's view is a preview of their app plus a change review (below).

Views solve your "in-platform browser?" question. You don't need a general browser; each view is served from the workspace through your platform's authenticated proxy and shown in a panel. This bends your "no servers" rule, and it's better to design for that honestly than to pretend otherwise: a prototype has to be served to be clickable. The distinction that keeps your security story intact:

- A view is reachable **only through the platform**, only by the owner or people they explicitly share it with.
- It exists only while being viewed, since opening it wakes the workspace.
- The workspace itself still accepts no inbound connections.

For the prototyper's presentations, add sharing: a private link, a public link toggle, or an export (static build, screenshots, a short recording).

**Anatomy of a workspace**

Every workspace can have the same five parts, with the AI deciding which to emphasize:

1. **Chat.** The primary way to direct work, like "Add saved searches to the app."
2. **Activity.** A plain-language timeline of what happened: "Ran 288 checks overnight, alerted you twice, one failure (API timeout, retried)."
3. **Views.** The generated interfaces above.
4. **Files.** A simple list, not a file manager, for things you bring in (a CSV) and things it produced (results, reports).
5. **Settings.** The details layer: schedules, notification channels, connections (GitHub, brokerage), secrets, resources, spend caps.

**The home screen**

The dashboard shouldn't be a grid of workspaces. It should answer "what needs me?" when you open it, in three sections:

- **Needs you:** a decision the agent is waiting on, a failure, changes ready for review.
- **Running:** "Stock watcher: live, last check 2 min ago."
- **Done recently:** "Real estate app: finished overnight, 4 changes pushed."

A single composer at the top ("What do you want to get done?") is the universal entry point. The finance bro types there without choosing a workspace first. The AI discusses the idea, then produces a **plan card**: "I'll create a workspace called Stock Watcher. It checks these tickers every 5 minutes and texts you when X happens. Estimated $6/month. Auto-start is on." The card has Approve and Edit buttons. Edit opens the same form someone would fill in manually, which keeps AI setup and manual setup as one thing.

**Presenting the founder's code changes**

A full IDE would be overkill, and it isn't what the founder wants at 7am anyway. A layered **change review** fits the rest of your design:

1. **Summary:** "Added saved searches, fixed the map pin bug, updated 2 dependencies." Each item includes the user feedback it responds to.
2. **Proof:** before-and-after screenshots and a live preview of the updated app (a view), plus test results.
3. **Changes by feature:** which files changed for each item, with readable diffs.
4. **Escape hatch:** open the pull request on GitHub, or open the code in their own editor.

Editing happens through chat ("make the saved-search button bigger"), with a read-only code browser in the details layer. One suggestion: even with auto-push on, have the agent push to a branch and open a PR by default, with "merge automatically if tests pass" as the opt-in. Waking up to a broken main branch is exactly the kind of experience that makes someone stop trusting overnight work.

**One caution on "trade on my behalf"**

Anything that moves real money needs hard guardrails built into the platform, not left to the agent: per-trade and daily limits, a confirmation mode ("text me, reply YES to execute"), and a paper-trading mode as the default. Beyond protecting users, there are regulatory questions around automated trading that you'll want a lawyer's view on before shipping it as a feature.

If it would help to see how this feels, I can mock up the two screens that carry most of the concept: the "needs you" home screen and a workspace page, using the finance bro or the founder as the example.


---

## User

One note on the views - while it's cool for AI to build, I would still want to follow a certain UI pattern - perhaps a library of components to keep things standardized. Anyway, mock up those screens


---

## Claude

I'll mock up the home screen and a workspace page, with views built from a standard component library.


I've made a design canvas with three screens. Two are the ones you asked for, and the third is the component library behind the views.

- **Home:** answers "what needs me?" when you open it. There's one composer at the top ("What do you want to get done?") with quick-start chips and a "Set up manually" option. Below it are three cards that each need a decision: the founder's overnight changes waiting for review, a data job paused by an API rate limit, and a brokerage connection about to expire. Then come "Running" and "Done recently" sections. A spend meter sits in the sidebar.
- **Workspace (Stock Watcher):** the view takes up most of the screen and chat stays docked on the right. The view is made from standard pieces: a row of key numbers, a watchlist table, a price chart with the alert threshold marked, and a plain-language list of what happened. In the chat, a request becomes a "proposed change" card with Approve and "Edit in form" buttons. That's what keeps asking the AI and setting things up by hand as one system. Paper trading is shown in the header rather than hidden in settings.
- **View components:** a fixed set of 12, grouped as Show, Collect and Act (for example stat, status, table, chart, progress, change review, file upload, form, "when this, do that" rule, action button). The AI picks components and connects them to the workspace's data, but can't invent new UI.

That rule has a useful side effect: a component like **Change review** can be designed properly once and then used everywhere, rather than the AI improvising a code-review screen for each project. It also makes views cheap to store and safe to render, because a view is just a list of components and their data sources, not generated code.

"Nightshift" is a placeholder name, and all the prices and numbers are made up. The link is private until you share it from the page.

The next screen worth mocking up is probably the founder's full change review, since that's where the IDE-or-not question gets settled.


---

## User

I like this diection so far. Let's mock up the full change review, and also the designer's prototype


---

## Claude

I'll add two screens to the canvas: the founder's change review and the designer's prototype workspace.

I added both screens to the canvas as a new row below the component library.

**Founder: change review.** My answer to "do we need an IDE?" is no, and this screen is the argument.
- **Left:** the overnight session at a glance: time, AI cost, tests passing, and the branch it pushed to. Below that are the 4 changes. Each one says which user feedback it came from and has a checkbox to keep it or leave it out. One is already left out: the AI flagged a major dependency upgrade as worth reviewing separately. That's why the merge button reads "Merge 3 of 4 changes."
- **Right:** the change you've selected, shown in layers:
  1. A plain-language description of what changed.
  2. Before-and-after screenshots, with a link to try it in the live preview.
  3. Proof chips: new tests pass, and it was clicked through in a real browser. A warning chip flags that it adds a database table, since that deserves attention.
  4. The code diff comes last.
- **Bottom:** a box for asking a tweak to this one change.
- The GitHub pull request is one click away for anyone who wants the full detail.

**Designer: prototype workspace.** This one is clickable: press Play on it, then either tap through the prototype itself or use the screen list on the left.
- **Centre:** the prototype, running on your platform rather than a general browser inside the app. It sits in a device frame you can switch between phone, tablet and desktop.
- **Notes for AI** is the new interaction here. The designer turns on note mode and clicks directly on the prototype to pin feedback. Each pin becomes a task, and the panel shows its status: done in version 3, or still in progress. For a designer that's more natural than describing a location in chat, though chat is still there.
- **Versions and Compare** sit in the header, because designers usually present iterations side by side.
- **Present** opens the prototype full screen without the platform's interface around it.
- **Share and export:** a share link that expires and points at one specific version, plus exports as a walkthrough video, screen images, or a code download.

One pattern now repeats on every screen: **the AI proposes and you approve**. In Stock Watcher it's the proposed-change card, in the change review it's the checkboxes, and in the prototype it's the notes with statuses. I'd make that a component in the library as well, since it's what makes overnight work feel safe to hand off.


---

## User

Great, this is along my general thinking as well, though some things need to be tweaked. Now, I want to explore manual usage of this platform, some people may not want to use AI. What are such use cases and how can we enable this?


---

## Claude

Manual use matters for more than people who dislike AI. For many of them it will be the main reason to pick your platform, and the design so far already supports it, because we decided early that **AI setup and manual setup produce the same thing**. Manual use is that same form and that same workspace, without the chat.

**Who wants to use it without AI**

1. **"I already have the code."** Engineers with a Python scraper, a Node sync script, or a Jupyter notebook. They don't want an AI rewriting it. They just want it running on a schedule, somewhere other than their laptop. This is probably the largest group, and it's you from your first message.
2. **Privacy and compliance.** Anyone whose data can't be sent to a model provider: client records, financial data, internal documents. "This workspace never sends anything to an AI" is a feature you can sell to them.
3. **People who want predictable results.** A price check should behave the same way every time, cost nothing in tokens, and never "interpret" anything. Many monitoring jobs need no intelligence at all.
4. **Non-technical people who just want a recipe.** "Tell me when this page changes" doesn't need a chat. It needs a form with three fields.
5. **People who bring their own agent.** They want to run Claude Code or another agent CLI themselves in a terminal that doesn't die when the laptop closes. From your platform's point of view that's manual use, even though AI is involved.
6. **Plain heavy computing.** Converting a folder of videos, backups and file syncs, a long data-processing run, or a chat bot that only reaches out to its service (Telegram and Discord bots work this way), which fits your "machines only dial out" rule.

**Three ways in, by skill level**

Each one is a door on the "Set up manually" path that's already on the home screen:

- **Templates (no code).** Ready-made tasks you configure with a form: watch a page for changes, check a price, back up A to B, send me a daily digest, run a batch over a file, keep a bot running. They use the same **Rule** and **Form** components from the library. Your team writes each template once as a reliable, AI-free runner, and users only ever see the form.
- **Bring your code.** Upload a folder, connect a GitHub repo, or paste a script. The platform reads what's there (`requirements.txt`, `package.json`, a Dockerfile) and pre-fills the task form: "Looks like Python 3.12. Run `python scrape.py`?" The user then answers the same five questions as always: when it runs, what it runs, what it remembers, where results go, and its limits. Secrets go in a vault and reach the script as environment variables.
- **Blank workspace with a terminal.** For experts: a terminal in the browser, reached through the same outbound connection, so the "no open ports" rule still holds. It's where people run their own agent CLIs, install tools, and debug. It belongs in the expert layer, and it's still not a desktop.

**The key piece: a small SDK**

Without AI, who builds the views? The answer is to **make the component library an output format**. A script reports its results through a tiny SDK, and the platform draws the standard components from them:

```python
from nightshift import stat, table, notify, progress, state

progress(done=1204, total=5000)
stat("Rows processed", 1204)
table("Results", rows)
if price < state.get("last_price"):
    notify(f"Price dropped to ${price}")
state.set("last_price", price)
```

That code would produce the same kind of dashboard the AI built for Stock Watcher, written by hand. It also covers the "what does it remember" question with a simple key-value store (`state`) and gives notifications one clear call (`notify`). For people who won't install anything, a fallback can work too: write a JSON file to a known folder, or print lines in an agreed format.

**For engineers: configuration as code**

- A `nightshift.toml` file in the repo holding the schedule, command, resources and limits. Pushing to GitHub updates the task, and the web form and the file stay in sync in both directions.
- A small CLI (`ns deploy`, `ns logs`, `ns run --now`) for people who live in their terminal.
- **"Run now" with test inputs** and a readable log view. For manual users, debugging is the whole experience, so the Activity tab needs raw logs and exit codes in its details layer.

**AI as optional, not assumed**

- An **AI off** switch per workspace, with a clear guarantee that nothing is sent to a model provider. It could also be an account-wide setting.
- If no AI key is connected, AI features are hidden rather than shown as locked.
- Handoff works in both directions: a manual task can later get "ask AI to add a chart," and an AI-built task can be taken over and edited by hand.

The screen that does the most for manual use is the **"new task, set up manually"** flow: choose a template or bring your code, then the five questions in the Apple-style layers, plus the Activity tab showing logs. I can add those to the canvas next if you want, or first work through the tweaks you mentioned for the existing screens.


---

## User

I like these ideas, and we'll definitely iterate on them. Before moving on, I want to explore competitors that already provide similar services


---

## Claude

I'll look up where the main competitors stand today, since this space has been changing quickly.

I looked at five groups of competitors. One of them is much closer to your idea than the others.

**The closest competitor: Manus Cloud Computer**

Manus launched it on April 30, 2026. It's an always-on Ubuntu machine in the cloud, run through plain-English instructions in the Manus app on web and mobile. It's aimed at non-technical users, and the work keeps running when their devices are off. Pitched uses include 24/7 Slack, Discord and Telegram bots, scheduled scrapers and reports, and self-hosting tools like Metabase or WordPress ([Eyerys](https://www.eyerys.com/articles/news/cloud-computer-use-manus-wants-users-have-agents-run-247-background)). It has no graphical desktop, but it does have a web terminal and SSH for manual use ([AI Automation Global](https://aiautomationglobal.com/blog/manus-cloud-computer-always-on-ai-automation-2026)).

That's your pitch almost word for word. The same review points out gaps you could use against it:
- There are three plan tiers, but no published prices.
- Little is said about security: nothing on inbound ports or public URLs.
- No SOC 2, ISO 27001 or HIPAA statements.

It's also one always-on machine per plan. There's nothing like your "workspaces that sleep" idea, and nothing like standard views built from a component library.

**The other groups**

| Group | Examples | What they cover | Where they fall short for your vision |
|---|---|---|---|
| **Cloud coding agents** | Claude Code on the web, Codex cloud, Cursor cloud agents, Jules, Devin, GitHub Copilot coding agent | The founder's scenario: a persistent cloud environment, runs while you sleep, opens a pull request automatically ([aq.dev](https://aq.dev/guides/best-cloud-coding-agents/), [amux](https://amux.io/guides/background-agents-compared/)) | Coding only. Mostly tied to one company's models. Built for engineers. |
| **App builders with scheduling** | Replit (Routines), Val Town | Replit's Routines repeat a task from a chat hourly, daily or weekly, using plain code for predictable steps and AI only when needed, with a budget per run ([Replit docs](https://docs.replit.com/references/agent/automations)). Val Town offers cron triggers and its Townie AI ([Val Town](https://blog.val.town/changelog-2026-09-10)). | Replit's Routines require paid plans, cap you at 5–10, and run no more than hourly. Val Town expects you to write code. Both are centred on building apps, not on running workloads. |
| **General AI agents** | Perplexity Computer | Long background tasks across many models, $200/month, launched February 2026 ([VentureBeat](https://venturebeat.com/technology/perplexity-launches-computer-ai-agent-that-coordinates-19-models-priced-at)) | Built around tasks, not persistent environments. No bring-your-own key. No manual path. |
| **Automation platforms** | Zapier Agents, n8n, Lindy, Gumloop | Non-technical automation between apps ([Gumloop](https://www.gumloop.com/blog/n8n-alternatives)) | Built around integrations ("when X, do Y"), not general computing. |
| **Single-purpose monitors** | Visualping, Distill, Browse AI | "Monitor anything and alert me," ready to use from about $10/month ([Visualping](https://visualping.io/blog/top-ai-website-monitoring-tools), [makerstack](https://makerstack.co/reviews/visualping-review/)) | One job only. |

A sixth group, **sandbox infrastructure** (E2B, Daytona, Modal, Fly Machines), is really a set of possible suppliers. Rather than building everything on Firecracker yourself, you could build on one of them at first ([Northflank](https://northflank.com/blog/daytona-vs-e2b-ai-code-execution-sandboxes), [AgentMarketCap](https://agentmarketcap.ai/blog/2026/04/07/ai-agent-sandbox-infrastructure-e2b-modal-daytona-fly-machines-secure-code-execution)).

**What this means for you**

1. **Coding alone isn't a big enough reason to switch.** Claude, OpenAI, Google, Cursor and GitHub all include cloud coding agents in plans people already pay for. That also changes the maths of bring-your-own key: a founder paying per API call may spend more than on a flat subscription that already includes a cloud agent. So your coding story has to be "any agent, next to all your other workloads, with a better review screen," not just "coding in the cloud."
2. **Your mix of features is still uncommon.** No one I found offers all of these together:
   - AI and manual setup producing the same editable thing
   - standard views built from a component library
   - workspaces that sleep when idle, rather than always-on machines
   - any AI provider through the user's own keys
   - a firm "nothing can connect in" security model
   - one home screen for every kind of workload

   Manus comes closest, and those are exactly its gaps.
3. **Trust could be the way in.** Manus publishes no prices or compliance statements. Your spend caps, clear prices, dry runs and the "AI off" guarantee answer a real weakness in the most direct competitor.
4. **Monitoring is a crowded market for a first product.** Visualping and similar tools already do "tell me when this page changes" cheaply. It makes a good template inside your platform but a weak headline.

My knowledge only runs to mid-2026 and these were quick searches, so it's worth trying Manus Cloud Computer and Replit Routines yourself before settling on your positioning. I can turn this into a competitive analysis doc if you want to keep it alongside the mockups.


---

## User

I've looked into manus and agree that it is close, however, it has its gaps. Regardless, I'm actually inspired by some of its features. For example, you can create sites/web apps and deploy on the computer, that's the in app browser feature I was mentioning earlier. It also allows you to host bots, those can be our templates. There are other features too, but I'd like to focus on ours.

There's also one more thing I want to build for us and that's collaboration. For example, two co-founders should be able to access their shared workspace from different places/devices, and we should be able to resolve conflicts. This could easily be achieved by letting one of them know the other already has something in progress. Collaboration is however not something to build initially, but it should factor into the design


---

## Claude

Both features fit the design, but hosting changes one of your founding rules, so it's worth deciding that on purpose.

**Hosting sites and apps**

Publishing a site means strangers' browsers make requests to something your platform runs. So "not a server" becomes "**not a server by default**." You can keep the security story intact with three rules:

- **The workspace still accepts no connections.** Public traffic arrives at your platform's edge and travels down the same outbound connection the workspace already opened, the way Cloudflare Tunnel works. The workspace never gets an open port or a public address.
- **Three levels of visibility, each a deliberate step up:**
  - **Private:** only you, which is the preview we already mocked up.
  - **Shared:** people you invite, who must sign in.
  - **Public:** anyone with the address.
- **Static sites go to a CDN (a network that serves files cheaply worldwide).** A prototype or landing page is just files, so the platform can copy them out and let the workspace sleep. Only dynamic apps need the workspace awake, and they can wake on the first request, accepting a slow first load, or pay to stay awake.

The real risk with public hosting is abuse. Free hosting on a trusted domain is exactly what phishing sites look for. Public publishing should need a verified account, and possibly a custom domain or a review step. That's another reason to keep it one level deeper than sharing.

**Bots as templates**

Bots fit your model better than hosting does. Telegram (long polling), Discord (its gateway connection) and Slack (Socket Mode) all let a bot reach out to the service instead of waiting for incoming requests, so no public address is needed. The catch is that a bot holding a connection keeps its workspace awake. The template should say that plainly: "This bot keeps its workspace awake, about $X/month." For services that support webhooks, the webhook can go to your platform instead, which wakes the workspace for each message. That's cheaper but slower to respond.

**Collaboration: what to build into the design now**

Your instinct ("tell the other person something is already in progress") is the right core. Here's what to put in place now so it's cheap to add later:

1. **Workspaces belong to an owner, not a user.** The owner can be a person or a team, and at launch every team has one member. Retrofitting ownership later is painful; starting with it costs almost nothing.
2. **Record who did everything.** Every run, chat message, change and approval gets an author. It's invisible when you're alone and essential with two people.
3. **Make the session the unit of work.** A session is one piece of work in progress, such as "Add saved searches," started by one person. This is where your conflict idea lives. When a co-founder opens the workspace, they see: "Ayo has a session running: *Add saved searches*, started 11:40 PM." They get three choices:
   - **Watch:** follow along live.
   - **Queue after:** start their request when Ayo's session finishes.
   - **Start in parallel:** run it alongside Ayo's.
4. **Keep parallel code work apart.** Each coding session works on its own branch, so git handles the merging, and a true conflict appears in the change review as something to resolve rather than as corrupted work.
5. **Warn on settings changes.** If two people change a schedule or rule at once, the second one sees "Ayo changed this 2 minutes ago. Review before saving." That avoids silently overwriting each other.
6. **One chat thread per session, not one per workspace.** Two people talking to one AI in the same thread gets confusing quickly. The Activity tab stays shared, and conversations belong to the session that started them.
7. **Send "Needs you" to the right person.** Whoever started the work gets the question, with a fallback to the rest of the team.
8. **Decide early:**
   - **Whose AI key pays?** Each member's own key, or a shared workspace key. This affects billing and spend caps.
   - **Who can see secrets?**
   - **Which actions need a second person's approval?** Merging to main and real-money trades are the obvious candidates, since two co-founders are exactly the case for them.

One more thing to build in from the start: the live stream of updates you already need for logs and progress is also what presence runs on ("Ayo is viewing"). If every screen already listens to that stream, adding collaboration later is mostly a matter of adding UI.

**Changes to the current mockups**

The single-person screens barely change:
- The composer and chat panel gain a small session header.
- Activity entries get author avatars.
- The home screen splits "Needs you" from "Waiting on your team."

I can add two screens to the canvas next if you want: the publishing and sharing flow for a site with the three visibility levels, and the "session already in progress" moment for the co-founders.


---

## User

For hosting sites and apps, I don't mean literally hosting them publicly, I mean hosting them privately on the computer (localhost), but allowing the user to actually use them through our platform. Sort of their own private site. They can choose to push the code to github and host somewhere else, that's up to them, but I think users should be able to interact with their site  as if they were testing through localhost on their computer

Bots as templates - note that templates aren't bots. Bots are just some built in templates we can offer.

Collaboration - I agree on your point.

Now that we have a baseline, I want to more concretely define our product. Our value prop, and an exhaustive list of all the features we offer. We should not go big at the start, I have lots of time to build this. I intend to start small with core use cases and expand. Don't just generate stuff, let's follow existing standards/structures for defining products like this. In fact, suggest these standards/structures and i'll decide on the right one


---

## Claude

Two corrections noted.

**Private sites on the workspace's localhost.** That's the right call, and it keeps "machines never accept connections" fully intact. Whatever the user's app is serving inside the workspace, say on port 3000, gets carried through the platform's authenticated connection and shown in a panel that behaves like a local browser tab. This should work for any local web interface, not just web apps: Streamlit and Jupyter notebooks, admin dashboards, API docs. Three technical details make it feel like the real localhost:
- live updates as you edit
- websockets
- sign-in cookies working correctly

Each preview should also get its own isolated web address, so one app can't read another's data. Publishing stays the user's own business: push to GitHub and host it wherever they like.

**Templates vs bots.** Understood. A template is the general mechanism: a pre-filled workspace or task. Bots are one category of built-in templates.

**Frameworks for defining the product**

These are the established options. They do different jobs, so the usual approach is one for the value proposition and one for the feature list.

*For the value proposition*

| Framework | What you get | Good for | Weak at |
|---|---|---|---|
| **Value Proposition Canvas** (Strategyzer) | For each customer type: the jobs they're trying to do, their pains and the gains they want, matched against how your product relieves the pains and creates the gains | Checking that every feature answers a real pain | Says nothing about scope or order |
| **Lean Canvas** (Ash Maurya) | One page: problem, customer segments, unique value proposition, solution, channels, revenue, costs, key metrics, unfair advantage | Treating it as a business, not just a product | Features get only one box |
| **Jobs to Be Done** (Christensen, Ulwick) | Job statements such as "When [situation], I want to [motivation], so I can [outcome]" | Customers who don't fit neat personas (the founder, finance bro and designer share jobs) | Can feel abstract without something to turn it into features |
| **Working Backwards PR/FAQ** (Amazon) | A one-page press release written as if the product has launched, plus customer and internal FAQs | Plain-language clarity, and forcing hard calls on what version 1 includes | Needs discipline to keep short |

*For the feature list and staging it*

| Framework | What you get | Good for | Weak at |
|---|---|---|---|
| **User Story Mapping** (Jeff Patton) | The user's journey as a backbone (set up, run, monitor, review, share), with every feature underneath, sliced horizontally into releases | A complete feature list and "start small, expand" in one picture | Less formal about requirements |
| **PRD** (product requirements document) | Problem, goals and non-goals, personas, user stories, requirements, success metrics, release plan | A detailed, conventional spec that engineering can build from | Tends to grow long, and prioritization is up to you |
| **MoSCoW** | Each feature labelled Must, Should, Could or Won't have (for now) | Quick, blunt scoping. Pairs with either of the above. | No sense of the user's journey |
| **Kano model** | Features sorted into basic expectations, performance features and delighters | Knowing what users will assume exists versus what makes them love it | Needs user research to be accurate |

**My recommendation**

**PR/FAQ for the value proposition, plus a User Story Map with MoSCoW-labelled release slices for the features.**

- The press release fits the Apple-style thinking you've been using. If you can't explain the product to a non-technical reader in one page, it isn't defined yet.
- The FAQ is where decisions like "not a server," bring-your-own key and the AI-off guarantee get written down.
- The story map gives you the exhaustive list you asked for, organised around what users actually do. The release slices mark the thin first version and the order you'll expand in, and a "Won't have for now" slice records things like collaboration and real-money trading, so they shape the design without being built yet.

A lighter alternative is **Lean Canvas plus a PRD**. It's quicker to write and more conventional, but weaker on staging.

Which combination do you want? I'll write it up as a document you can keep editing, filled in from what we've settled so far and with open decisions flagged instead of guessed.


---

## User

I agree with PR/FAQ for the value proposition, plus a User Story Map with MoSCoW-labelled release slices for the features.


---

## Claude

Outline is up; I'll fill in the press release first.

Now the press release.

Next, the customer FAQ.

Now the internal FAQ: principles, architecture, competition and risks.

Now the backbone of the story map, drawn as a diagram.

Next, the full feature list under each activity. I'll add it in two parts.

Now the release slices, drawn as a roadmap.

The product definition doc is ready, with the PR/FAQ, the story map and the release slices. Priorities and releases are dropdowns you can change in place. I left one comment asking whether private previews should move into R1. Reply there or comment anywhere and I'll revise.


---

## User

Let's address the open decisions.

1. Nightshift.com is not available, so we need a new name. Ideally one that has free .com, .io, etc. I imagine the app itself will be at an "app.businessname.extension"
2. Technical founders can be our first customer since I have 3 of those lined up already. However, I still want to include some non-technical features in R1
3. R1 agent. Let's start with claude code. The setup should be as straightforward as possible.
4. Compute - not decided yet. We will need to discuss technical architecture first.
5. Pricing - This deserves its own discussion, but there are many ways we could approach this.
6. AI subscriptions - If we can allow users to sign in, that would be great
7. Retention - We should have defaults that can be reconfigured by users
8. Gate targets - not sure what these mean
9. Launch templates - We can iterate on 10-20 templates, bots included.
10. Regions - I want to default to just the US for now. EU and other countries have their own data privacy laws. In fact, this platform should only be available in the US at launch, then we can expand based on demand


I like our product definition now. After addressing my open question responses, what should we tackle next?


---

## Claude

I'll check two facts first: whether users can sign in to a Claude subscription inside our workspaces, and whether I can check domain availability from here.

I've updated the doc's Decisions section with your answers. It also notes the subscription conditions, proposed retention defaults, a proposed first gate, and a new FAQ entry saying the product is US-only at launch.

**1. Name.** I checked domain registration records directly, and these names showed no registered domain on .com, .io, .dev, .ai or .app:

| Name | Why it works |
|---|---|
| **Lidclosed** | The whole pitch in one word: "Close the lid. It keeps going." |
| **Lidlater** | Lighter and friendlier, with a nod to "see you later" |
| **Latelit** | A lit window late at night: work quietly going on |
| **Lamphour** | The hours you'd normally be working by lamplight |
| **Burnlow** | Steady, low-cost work that keeps going |

Lidclosed is my pick because it describes the product for non-technical users without any explanation. No DNS record doesn't guarantee a domain can be bought, since some are held by resellers. Check with a registrar, and run a USPTO trademark search before buying.

**2. Non-technical features in R1.** You've already moved several stories into R1: templates, dashboards and private previews among them. The release-slices drawing still shows the original plan, so I can redraw it to match your edits.

**6. AI subscriptions.** Signing in is allowed, with conditions from [Anthropic's Claude Code terms](https://code.claude.com/docs/en/legal-and-compliance):
- Users sign in to the unmodified Claude Code with their own subscription, and the sign-in finishes on Anthropic's own page. We must never touch their credentials.
- We can't offer our own "Sign in with Claude" button.
- We have to accept Anthropic's Commercial Terms to host Claude Code.

That still fits your "straightforward setup" goal: a "Connect Claude" step starts Claude Code's own sign-in and shows its link. The tradeoff is that subscription credentials live inside the workspace, while API keys stay in our proxy. Also, since May 2026 Anthropic caps some programmatic subscription use at separate monthly credits ([VentureBeat](https://venturebeat.com/technology/anthropic-reinstates-openclaw-and-third-party-agent-usage-on-claude-subscriptions-with-a-catch)). Long overnight sessions may hit subscription limits, so it's worth testing with your three founders.

**8. Gate targets.** A gate is the evidence you need before starting the next release, so you don't build R2 on top of an R1 that isn't working yet. I proposed one for R1 to R2: all three alpha founders use it at least weekly for four straight weeks, with no lost work and no surprise bills.

**What to tackle next**

1. **Technical architecture, next.** It's the biggest unknown blocking R1, it settles the compute decision, and pricing depends on what compute actually costs. I'd make it a design doc covering how workspaces sleep and wake, the outgoing-only connection, the key proxy, how private previews reach the browser, and whether to build compute yourself or rent it.
2. **Pricing, straight after,** once there are real cost numbers from the architecture.
3. **In parallel:**
   - Settle the name, buy the domains and file a trademark search.
   - Interview your three founders about their overnight work, before building. What they actually hand off will sharpen R1 more than anything else.

Shall I start the architecture doc?


---

## User

let's tackle the name first. Everything I've looked up is taken. I want you to explore some (or combinations) of these ideas, i give an example for each which is already taken. No need to check for domain, I'll do that my self.

1. A "box" or "container", e.g boxy
2. Something that happens at night - nighshift, overnight, overwatch
3. Something in the cloud - mycloud
4. Something personal - mybox

let's tackle the name first. Everything I've looked up is taken. I want you to explore some (or combinations) of these ideas, i give an example for each which is already taken. No need to check for domain, I'll do that my self.

1. A private "box" or "container", e.g boxy, privco
2. Something that happens at night - nighshift, overnight, overwatch
3. Something in the cloud - mycloud, otherbox
4. Something personal - mybox
5. Something that abstracts details away - netherbox, 
6. something computer related - computeaway
7. Something related to a feature of our app - e.g windows represents the windows in the OS, temporal represents time-scheduled tasks,
8. something representing continuation - e.g continuum
9. Something completely unrelated but simple - e.g Apple, Oracle


---

## Claude

Here are candidates for each of your nine directions, then combinations and my shortlist. I've skipped names I know are big brands, and I flag known clashes in similar products where I'm aware of them.

**1. A private box**
- **Coffer:** a strongbox for valuables. Private and a little premium.
- **Cubby:** a small space of your own. Friendly for non-technical users.
- **Hutch:** a small, sheltered home.
- **Keepsafe:** where you keep things safe. It may clash with existing privacy apps.
- **Cove:** a sheltered, private inlet.

**2. Happens at night**
- **Nightjar:** a bird that's active at night. Distinctive and easy to remember.
- **Lamplight:** working by lamp after everyone's gone home.
- **Gloaming:** the fading light at dusk. Beautiful, but harder to spell.
- **Owlet:** a young owl. Small and friendly.
- **Midnight Oil:** from "burning the midnight oil." Too long for a product name, but good for a tagline.

**3. In the cloud**
- **Aerie:** a nest built high up. A private place in the sky.
- **Cirrus:** high, wispy clouds. Light and calm.
- **Updraft:** rising air that carries things upward.
- **Overhead:** it's up there and working.

**4. Personal**
- **Hearth:** the heart of a home, and warm.
- **Den:** your own private room.
- **Kin:** short and warm, though it hints at family rather than work.

**5. Hides the details**
- **Offstage:** the work happens out of view, and you see only the performance. My favourite in this group.
- **Understudy:** someone who quietly steps in and does the job.
- **Stagehand:** the perfect meaning, but Browserbase already has an AI browser tool called Stagehand, which is close to your space.
- **Backstage:** also a good meaning, but Spotify's developer portal already uses the name.

**6. Computer-related**
- **Uptime:** an engineering word for "still running." Plain to engineers, and understandable to everyone else.
- **Daemon:** the technical term for a program running in the background. Perfect for engineers, but it reads as "demon" to everyone else.
- **Runloop:** a loop that keeps going.
- **Headless:** a computer with no screen, which is literally your product. The name is a bit odd, though.

**7. A feature of the app**
- **Baton:** you hand off work like a relay baton, and it keeps moving. This covers your handoff feature and the continuation idea at once.
- **Pane:** a window pane, from the private previews. A nod to operating systems without saying "Windows."
- **Cadence:** scheduled, rhythmic work.
- **Stencil:** from the templates. Weaker, since templates aren't the core.
- **Nudge:** from the "needs you" moments. Friendly, but it undersells the product.

**8. Continuation**
- **Onward:** simple, and it moves forward.
- **Relay:** work passed along and carried on. Common, so probably taken widely.
- **Perennial:** it keeps coming back.
- **Unbroken:** work that doesn't stop when you do.
- **Evergreen:** always on, always fresh.

**9. Simple and unrelated**
- **Heron:** a patient bird that waits, then acts.
- **Kettle:** it keeps things warm while you're away.
- **Juniper, Fern, Pebble, Acorn:** soft, memorable and neutral.

**Combinations**
- **Nightloft:** night, a private room and somewhere up high. It covers three of your ideas.
- **Owlbox:** a nest box for owls. Night plus private box, and playful.
- **Duskbox:** a box that works from dusk.
- **Moonhutch:** night plus a private home.
- **Lampbox:** a lit box working late.
- **Offstage Cloud:** as a fallback if Offstage alone is taken everywhere.

**My shortlist**
1. **Baton.** Instantly explains the handoff, it's short, and non-technical people get it.
2. **Nightloft.** Night, private and in the cloud in one warm word.
3. **Offstage.** "Your work happens offstage" is a strong line, and it matches the idea of hiding the details.
4. **Nightjar.** Distinctive, about night, and easy to build a brand and logo around.
5. **Pane.** Short, tied to a real feature, and it nods to operating systems.

A quick way to choose: say each one in a sentence a customer would use, like "I left it running on Baton" or "Nightloft finished it overnight." The one that sounds most natural usually wins. Want me to explore any of these further, in variations or with a tagline for each?


---

## User

let's focus on "box". give ideas


---

## Claude

Here are box ideas, grouped by angle, then my shortlist. One caution first: "-box" names sit near **Box** and **Dropbox**, both large cloud-storage brands with active trademarks. A name that doesn't end in "box", or that uses a specific kind of box, will be easier to register and to tell apart.

**Types of box, used as metaphors**
- **Bento:** a Japanese lunch box with compartments, one for each thing. That maps neatly onto workspaces. It's friendly, simple and visual. The link-in-bio product Bento, acquired by Linktree, used the name, so check it.
- **Coffer:** a strongbox for valuables. Private and a little premium.
- **Caddy:** a tea caddy keeps things fresh, and a golf caddy carries the load for you. Both meanings fit.
- **Locker:** your own space in a shared building. Very clear, but common.
- **Cubby:** a small space that's yours. The friendliest of the group.
- **Passbox:** a hatch in labs and clean rooms that you pass things through without going inside. That's exactly your "nothing connects in, work passes through" model.
- **Tinderbox:** where a spark starts something. Energetic, though it can also suggest "about to catch fire."
- **Music box:** it keeps playing after you walk away. Too long as a name, but it'd make a lovely brand story.
- **Signal box:** the railway cabin that quietly keeps trains running. A nice meaning, but obscure.

**"Box" combined with your other themes**
- Night: **Owlbox**, **Duskbox**, **Moonbox**, **Nightbox**, **Latebox**
- Cloud: **Nimbox** (nimbus + box), **Cloudbox**, **Airbox**
- Lid closed: **Lidbox**. It ties back to "close the lid, it keeps going."
- Continuation: **Runbox**, **Keepbox**, **Onbox**
- Personal: **Ownbox**, **Homebox**
- Hidden: **Hushbox**, **Quietbox**

**"Box" in other languages**
- **Hako:** Japanese for box. Short, soft and easy to say.
- **Kasten:** German. Solid and engineered-sounding.
- **Kutu:** Malay and Indonesian. Playful.
- **Doos:** Dutch. Probably too close to "doze," though that could also be a clever nod to sleeping.

**Playful spellings**
- **Boxel:** box plus pixel or voxel. Techy but friendly.
- **Boxen:** an old-fashioned plural, like oxen. Hackers already use it jokingly.
- **Boxly:** easy, but generic.

**My shortlist**
1. **Bento.** Compartments for each piece of work, it's warm, and it's easy to design for. "My scraper's running in my Bento."
2. **Passbox.** The most meaningful: work passes in, results pass out, and nothing gets inside. Security-minded without sounding scary.
3. **Hako.** Short and distinctive, and it avoids the crowded "-box" suffix while still meaning box.
4. **Coffer.** Private and trustworthy, and it doesn't sound technical.
5. **Lidbox.** The most on-message for the pitch, and a bit playful.

To choose, try each in the sentence "I left it running in my ___ overnight." Bento and Coffer read the most naturally there. I can write taglines and a one-line brand story for your top two or three if that helps.


---

## User

think of things from the greek alphabet, the periodic table, elementary particles, mythology


---

## Claude

These four sources work well together, because several words cross between them: some elements are named after mythological beings, and some particles after Greek letters. I've given each name's meaning and how it fits, and flagged clashes I know of with tech products. You'll still want to check trademarks.

**Mythology**

*Night, sleep and dawn*
- **Nyx:** Greek goddess of night. Short and striking. It's popular in tech, so expect clashes.
- **Hypnos:** god of sleep. It fits "workspaces sleep", but sounds a bit like hypnosis.
- **Selene:** goddess of the moon. Soft and elegant.
- **Hesperus:** the evening star, which appears as the day's work ends.
- **Eos:** goddess of dawn. "Wake up to finished work." Short, and a strong story.

*Watchers and helpers*
- **Argus:** the giant with a hundred eyes who never fully slept. A perfect story for monitoring.
- **Talos:** the bronze automaton that guarded Crete, often called the first robot. Tireless work.
- **Ariadne:** her thread guided Theseus through the labyrinth. Continuation, and finding the way through.
- **Kairos:** the right moment. Fits scheduled and triggered work.
- **Mnemosyne:** goddess of memory. Great meaning, impossible to spell.
- **Hestia:** goddess of the hearth and home. Warm and personal.

*Folklore helpers who work while you sleep.* These are the most on-message names on this list.
- **Nisse / Tomte:** Scandinavian farm spirits who quietly do chores at night, if you treat them well.
- **Kobold:** German household and mine spirits who help with work.
- **Brownie:** the Scottish house spirit who finishes the chores overnight. It will always sound like the dessert, though.
- **Domovoi:** the Slavic house spirit who keeps the home running.

*Avoid.* These are heavily used in tech or carry the wrong meaning:
- **Prometheus:** a widely used monitoring tool.
- **Atlas:** MongoDB's cloud database.
- **Hermes:** the fashion brand.
- **Janus:** two-faced.
- **Pandora:** a box, but full of troubles.

**Periodic table**
- **Argon:** from the Greek *argos*, meaning "idle." An element named for resting fits a workspace that sleeps until needed, and argon is also known for being stable and unreactive, which suits "safe." Argo CD is a well-known developer tool, so check for confusion.
- **Cobalt:** named after the kobold spirits in German mines. It connects the element to the folklore helpers above, and the colour is a natural brand colour. Common as a company name.
- **Palladium:** named after the Palladion, a statue believed to protect the city of Troy. That suits "private and protected," though it's long.
- **Xenon:** from the Greek for "stranger." Striking, but the meaning doesn't fit a personal product.
- **Iridium:** from Iris, the messenger goddess. It's also a satellite phone company.
- **Neon:** avoid. Neon is a Postgres database that famously sleeps when idle, which is too close to your product.

**Greek alphabet**
- **Delta:** the symbol for change, which is your change review. Taken by the airline and several developer tools.
- **Kappa, Theta, Zeta, Iota:** short and neutral. On their own they carry no meaning for your product.
- **Omega:** the last letter, meaning "it finishes."
- **Lambda:** avoid, because of AWS Lambda.
- Letters also combine well with the other ideas: **Deltabox**, **Thetabox**, **Kappa Cloud**.

**Elementary particles**
- **Muon:** short, soft and unusual.
- **Gluon:** the particle that holds things together. Could suit a platform that ties your work together.
- **Photon:** light itself, which fits a light always on. Widely used in tech already.
- **Neutrino:** passes through everything unnoticed, like work that happens without you seeing it. Long.
- **Boson:** a solid physics name, with nothing specific tying it to your product.
- **Quark:** playful, but used widely.

**My shortlist**
1. **Eos:** "Go to sleep. Wake up at Eos." Short, it tells the core story, and it's easy to design for.
2. **Argon:** the element named for idleness, for workspaces that rest until needed. The story is clever and quietly technical.
3. **Nisse:** the night helper who does your chores while you sleep. The most on-message folklore name, and it's warm and unusual.
4. **Argus:** the watcher who never sleeps. Strong for monitoring, though that's narrower than your whole product.
5. **Cobalt:** it ties the element to the helper spirits, and comes with a ready-made brand colour.

Eos and Nisse both describe the core promise best: work that happens overnight while you rest. Want taglines and a one-paragraph brand story for your top picks?


---

## User

croncave is free. give me other cron-related ideas


---

## Claude

Croncave is a nice find: the alliteration makes it memorable, and a cave suits "private and out of sight." Here are more ideas in the same vein.

**Cron plus a place, like Croncave**
- **Croncove:** a sheltered private inlet. Softer and more inviting than a cave.
- **Cronloft:** a private room up high, echoing the cloud.
- **Cronden:** your own den.
- **Cronnest:** a safe home for your work. Friendly.
- **Cronhive:** busy workers inside one home. Fits several tasks per workspace.
- **Cronkeep:** a castle's keep, the strongest and most protected tower.
- **Cronyard, Cronshed, Cronhut:** homely and practical.
- **Cronworks / Cronforge:** a place where things get made. Suits the building side, like coding sessions.

**Cron plus night**
- **Cronowl:** the night-owl scheduler. Playful, and easy to put in a logo.
- **Nightcron:** literal and clear.
- **Cronmoon / Cronlight:** gentler and more atmospheric.

**Cron plus continuation**
- **Cronward:** cron plus onward.
- **Cronon:** "cron on," as in keep going. Reads well written down, less well out loud.
- **Cronever:** runs forever.

**From Chronos, the Greek god of time and the origin of "cron"**
- **Chrono:** simple, and friendlier to non-technical users than "cron."
- **Chronicle:** a record of what happened, which is your activity timeline. Google already uses it for a security product.
- **Kairon:** from Kairos, the right moment. A fitting pair for scheduling.

**From cron's own syntax**
- **Fivestar:** a cron schedule is five fields, and `* * * * *` literally means every minute.
- **Crontab / Tabcave:** in-jokes for engineers.

**Short coined forms**
- **Cronly, Croni, Crona, Cronix:** easy to register, but with little meaning of their own.

**Avoid**
- **Crony:** fun, but "cronyism" is negative.
- **Cronicle:** an existing open-source cron scheduler.
- **Cronut:** a trademarked pastry.
- **Micron, Macron:** a chip company and a French president.

**How cron names fit the product**

Engineers will get "cron" instantly, which suits your first customers. Most non-technical people won't know the word, and it only really describes scheduled tasks, not overnight AI sessions or private previews. Neither is a dealbreaker: a playful second half like cave, owl or cove carries the name for everyone else, the way "Dropbox" works without anyone knowing what a drop box is.

**My picks:** Croncave, Cronowl and Cronloft. Croncave and Cronowl are the most memorable, and Cronloft is the easiest to explain ("your private loft in the cloud").


---

## User

Let's go with croncave for now. We can change it later. Now let's iterate on the technical architecture


---

## Claude

The outline is up. I'll start with goals and non-goals.

Next is the system overview diagram.

Now I'll research the compute options for the build-vs-rent comparison.

Next, the workspace lifecycle diagram.

Next, the outgoing connection.

Now private previews.

Now the scheduler, runs and events.

Now the data model and storage.

Now security and abuse controls.

Next, the stack summary and build order.

Last section: open questions.

The architecture doc is drafted, with diagrams for the system and the workspace lifecycle. My main recommendation is to rent compute for R1, starting with Fly Machines, behind an interface that lets you move to your own servers later. Edit it directly or comment, and I'll revise.

The product definition still uses "Nightshift" as the name. Want me to rename it to Croncave there too?


---

## User

I added some comments to the doc, let's discuss them before you make changes. Reply to the comments and let me know when you are done. If all looks good, I'll ask you to make the updates


---

## Claude

Before replying I'll check where the rental providers run their hardware and how they handle resizing.

I've replied in each of your comment threads. The API-key gateway is the only one that needs your decision; the rest are ready to apply.


---

## User

I've updated the comments. We should be able to finalize all the open questions now.


---

## Claude

I'll apply the agreed changes section by section, starting with the wake-time goal and the compute section.

Now the lifecycle rules and the previews decision.

Now AI access and GitHub.

Now the scheduler, data model, security, stack and decision list.

I've updated the doc and replied to each thread. Every open question is now decided except the preview domain. `preview.croncave.com` isn't safe on its own, so I suggested an alternative in that thread for you to confirm.


---

## User

sounds good, agreed on that


---

## Claude

I've written the preview domain decision into the doc, so every open question is now settled.


---

## User

what next?


---

## Claude

Planning is nearly done; what's left is mostly building. Here's the order I'd suggest.

1. **Pricing, next.** It's the last big planning decision, and the architecture now gives us what it depends on: awake time plus disk, packing in R2, and the idle timeout. We'd need Fly's actual prices for a few workspace sizes. The output is a cost model, what to charge, and how spending caps and the free tier work.
2. **Talk to your three founders, alongside pricing.** A 30-minute call each about what they'd hand off overnight, how they'd judge the result, and what they'd pay. That checks both the R1 scope and the pricing before you build.
3. **Housekeeping, which I can do now:**
   - Rename Nightshift to Croncave throughout the product definition.
   - Redraw its release-slices diagram to match your R1 edits.
   - Link the two docs to each other.
   - Register croncave.com and the preview domain.
4. **Then start building, beginning with the riskiest part.** Before the full skeleton, I'd prove the core idea end to end: a Fly machine, the Rust agent dialling out to a relay, and a terminal in a SvelteKit page. That's step 3 of the build order. If it works smoothly, everything else is familiar web and backend work. If it doesn't, you find out in week one.

My suggestion is housekeeping now, since it's quick, then pricing. Should I start?


---

## User

1. I'm thinking of going with subscription tiers which allow various features. A free tier, a paid tier, and a pro tier. All tiers have usage-based billing in addition to the base subscription. Tiers can vary by many things like number of allowed workspaces, tasks, max storage, max compute, AI usage caps, template access, and many more. Usage-based billing is exactly the costs from downstream providers (fly, e2b, our AI-feature provider), plus a small percentage.
2. I already know what they want, it's part of our features already.
3. Yes do the housekeeping, don't register the domain. I'll do that manually
4. Before building, I want to explore use of open-source models, both for our in-product AI features and as options for Agentic workflows.


---

## Claude

I'll do the housekeeping on the product definition first, then research open-source models.

I've read your current priorities. Now I'll update the text, rename to Croncave, and redraw the release slices to match your edits.

The product definition is updated. Now I'll research open-weight models, the agents that can use them, and where to host them.

I've done the housekeeping. The product definition now says Croncave throughout, and the release-slices drawing matches your R1 changes (previews, dashboards and templates moved into R1). The pricing direction is recorded, and the two docs link to each other. The doc title is renamed too.

**Open-source models**

These two uses have different needs, so I'll take them separately.

**1. AI features inside the product**

Examples: turning a chat into a task plan, generating dashboard layouts, writing summaries of changes, filling in templates, and spotting broken scrapers. Most of these are short, structured jobs ("turn this request into this JSON"). Mid-sized open models handle that well, and much more cheaply.

- The best open models now cost roughly a tenth of what top closed models charge for output. For example, DeepSeek V4 Flash is about $0.14 input and $0.28 output per million tokens, and V4 Pro about $0.44 and $0.87 ([Morph](https://www.morphllm.com/best-open-source-coding-model-2026)).
- Hosted inference providers such as Together, Fireworks, DeepInfra and Groq serve these models through OpenAI-compatible APIs ([CloudZero](https://www.cloudzero.com/blog/llm-inference-providers/)). So we don't need our own GPUs to start.
- **Recommendation:** put a small model router behind these features. Default to a cheap open model, and send the hard cases, like complex plans, to a stronger model. Build a small evaluation set of real requests, so switching models is a measured decision rather than a guess.

**2. Open models for agent sessions**

- **Claude Code can't use them.** Its docs say it doesn't support routing to non-Claude models through a gateway ([Claude Code docs](https://code.claude.com/docs/en/llm-gateway)). Open models therefore need a second agent program.
- **Candidates for that second agent** ([Pinggy, Aug 2026](https://pinggy.io/blog/best_open_source_cli_coding_agents/)):
  - **OpenCode:** MIT licence, works with any provider, runs without a person present.
  - **OpenAI's Codex CLI:** Apache licence, supports open models.
  - **OpenHands:** MIT licence, works with 100+ providers.
  - **Goose:** Apache licence, suited to general tasks beyond coding.

  My pick is OpenCode. It fits the "choice of agents" item already planned for R2, and it can point at our same AI gateway.
- **The quality gap is real for unattended work.** On SWE-bench Verified, a common coding benchmark, DeepSeek V4 Pro scores about 81% and GLM-5.2 about 79%. Top closed models score about 95–96%. Kimi K3 is the exception at about 93%, though it has a custom licence ([Morph](https://www.morphllm.com/best-open-source-coding-model-2026)). For overnight coding, I'd offer open models as a cheaper option and keep Claude Code as the default.

**The biggest risk is where the strongest open models come from**

Most of them come from Chinese labs, and US policy around them is getting stricter:
- The FY2026 defence spending bill removes DeepSeek from Defense Department and contractor systems.
- Alibaba, which makes Qwen, was added to the Pentagon's list of Chinese military companies in June 2026.
- The State Department warned against using Chinese models in July ([TechTimes](https://www.techtimes.com/articles/320171/20260711/washington-wants-chinese-ai-out-corporate-america-open-weights-block-ban.htm)).
- Treasury threatened sanctions over distillation, though none had been imposed as of late July ([Vibranium Labs](https://vibraniumlabs.ai/blog/chinese-ai-models-sanctioned-whats-actually-happening)).

Using these models is legal today, but it could change quickly, and your US-only customers may include government contractors. So:
- **Default in-product features to US-origin open models.** Options include OpenAI's gpt-oss, Nvidia's Nemotron 3, Arcee's Trinity and Poolside's Laguna, all under permissive licences ([Layer3 Labs](https://www.layer3labs.io/open-weights/american-open-weight-ai-models)).
- **Offer Chinese-origin models only as a choice users make,** clearly labelled, and served by US inference providers.
- **Keep everything behind the router,** so a model can be switched off in minutes.

**One conflict to resolve**

Your pricing plan bills AI usage at provider cost plus a percentage. The product definition's "Won't have for now" list says "no reselling AI usage." For Claude that stays true, because Anthropic's terms forbid it. For open models, reselling is allowed and fits your pricing model well. We'd just narrow that line to "no reselling Claude usage."

Where we could go next:
- Add a "Models" section to the architecture doc: the router, the second agent, and the provider allowlist, limited to US-hosted providers that don't keep our data.
- Start the pricing doc, now that the costs are clearer.


---

## User

For AI usage, let's keep it simple and remove the percentage from provider cost. I agree with the model router. For open models, yes I know about the chinese model conversations and I agree that these should be opt-in rather than opt-out. I've used Kimi K3 and would want to start with that for both coding an in-product features. We can add more models and coding agents later one


---

## Claude

Before updating the docs I'll check two things about Kimi K3: its licence terms and which US providers host it.

I've updated the product definition: usage is billed at exactly our providers' cost with no markup, and "no reselling" now applies only to Claude. Before I write Kimi K3 into the architecture, three things need your attention.

**1. Kimi K3 as the default conflicts with "opt-in"**

Kimi K3 comes from Moonshot AI, a Chinese lab. If it powers the built-in AI features by default, every user is using a Chinese-origin model without choosing to. That's opt-out, not opt-in.

Running it through US providers reduces the data concern. Fireworks and Baseten both serve Kimi K3 from the US with zero data retention ([Vercel](https://vercel.com/changelog/kimi-k3-and-kimi-k3-fast-on-ai-gateway)), so prompts never go to Moonshot. The policy risk stays, though: Moonshot's July launch of K3 is what prompted Treasury's sanctions warning ([Vibranium Labs](https://vibraniumlabs.ai/blog/chinese-ai-models-sanctioned-whats-actually-happening)). Two ways to square it:
- **Kimi K3 as the default for built-in features, clearly disclosed,** with a one-click switch to a US-origin model and the router ready to swap it out overnight if needed.
- **A US-origin model as the default, with Kimi K3 opt-in** everywhere, which is stricter and closer to what we agreed.

For coding sessions, Kimi K3 is naturally opt-in, since users pick it as their agent.

**2. Kimi K3 isn't cheap**

It's about $3 per million input tokens and $15 per million output on Fireworks ([Fireworks](https://x.com/FireworksAI_HQ/status/2081764194085691449)). That's frontier-level pricing, not the "tenth of the cost" I quoted for cheaper open models. Users pay it at cost, so it isn't our margin, but the built-in features run often and could make a free tier feel expensive.

**3. Licence and agent notes**

- **Licence:** commercial use is allowed. Showing "Kimi K3" in the product only becomes mandatory above 100M monthly users or $20M monthly revenue. A separate agreement is needed for "model as a service" businesses above $20M in revenue ([Kimi K3 licence](https://huggingface.co/moonshotai/Kimi-K3/raw/main/LICENSE)). Use through Moonshot's certified inference partners is exempt from those clauses, so it's worth confirming whether Fireworks or Baseten qualify.
- **Which agent runs Kimi for coding:**
  - Claude Code doesn't support non-Claude models, even though Moonshot documents using it that way.
  - Moonshot's own Kimi Code CLI is MIT-licensed, but its docs don't mention a mode for running without a person present ([GitHub](https://github.com/MoonshotAI/kimi-code)).
  - For unattended overnight sessions, OpenCode is the safer choice.

**A side effect of dropping the percentage**

Card processing fees (typically around 3%) apply to usage charges too, so passing costs through at exactly cost loses a little on every usage dollar. The subscription tiers need to cover that. The simplest fix is folding it into subscription prices rather than adding a markup back.

Which way do you want the built-in AI features to go: Kimi K3 by default with a switch, or a US-origin default with Kimi K3 opt-in? Once you've decided, I'll add a Models section to the architecture doc covering the router, the providers, OpenCode as the second agent, and the opt-in rules.


---

## User

I meant kimi as the first open model we support for opt-in, we still want to use US-origin models as the default. And yes we should run it through US providers. However, since Kimi is not cheap, let's go with the next best model then.

Note that we are only dropping the percentage for AI usage - we still want a percentage for everything else where allowed


---

## Claude

I'll check which models fit each role before writing the Models section.

I've updated both docs.

- **Pricing** (product definition): AI usage is billed at exactly what the provider charges us. Everything else, like compute and storage, is provider cost plus a small percentage, where the provider's terms allow a markup.
- **New "Models and the router" section** (architecture doc):
  - **Built-in AI features** default to US-origin open models: Nvidia Nemotron 3 Ultra for harder requests, and a smaller US model for quick ones. The final picks come from testing them on real requests.
  - **Coding sessions** stay on Claude Code by default.
  - **Opt-in open model:** DeepSeek V4 Pro, the next best open coding model after Kimi K3. It scores about 81% on SWE-bench Verified versus Kimi's 93%, but costs about $0.44 input and $0.87 output per million tokens, against Kimi's $3 and $15. It runs in OpenCode, since Claude Code only supports Claude models.
  - **Router rules:**
    - Calls go only to US-hosted providers that don't keep our data.
    - Every model is labelled with where it comes from, and Chinese-origin models stay off unless a user opts in.
    - Any model can be switched off in minutes.
    - A model only replaces another after passing our tests.
  - **Timing:** the router and the US default models are needed in R1, because plan cards and dashboards are R1 features. OpenCode and DeepSeek arrive in R2.

**Two things I added that you should check:**
- **DeepSeek carries a warning.** The FY2026 defence spending act removes DeepSeek from Defense Department and contractor systems, so the opt-in screen warns government contractors.
- **GLM is excluded.** It would have been a close alternative, but its maker, Zhipu, is on the US Commerce Department's Entity List.

Next could be the pricing doc: tier limits and features, plus how card fees fit when AI usage is passed through at cost.


---

## User

Hmmm, I'm a bit concerned about the non-US open models and their warnings. At this point, I think we should just use only US-originated models. Default to the high quality closed models (claude), and allow opt-in to the US-open models.

For R1 however, everything should just be claude


---

## Claude

I've updated both docs: models are now US-origin only, Claude is the default, and R1 uses Claude and nothing else.

- **Architecture doc:**
  - **Built-in AI features** call Claude through Croncave's own Anthropic API account and are billed to users at exactly our cost. A small, fast Claude model handles quick tasks and a stronger one handles plans; which model does what gets settled by testing on real requests.
  - **Coding sessions** stay on Claude Code, with the user's API key or subscription.
  - **From R2**, users can opt in to US-origin open models such as Nvidia Nemotron 3 Ultra, running in OpenCode. DeepSeek and Kimi are removed.
  - **The model router still ships in R1 with only Claude behind it**, so adding open models later needs no redesign.
- **Product definition:** "no reselling" now refers specifically to users' Claude Code usage, and "AI models from non-US labs" is on the Won't-have list.

One thing to confirm with Anthropic. Anthropic's no-resale rule covers Claude Code usage on a user's behalf. Calling Claude from our own API account for built-in features, and billing that at cost, should count as ordinary API use under their Commercial Terms. I've added it to the gateway question we're already asking them, so it gets answered at the same time.

The pricing doc is the natural next step.


---

## User

Create the pricing doc. Once that's ready, I'm ready to explore implementation


---

## Claude

The outline is up. I'll write the summary, then look up provider prices.

Now the free-tier guardrails.

The pricing doc is drafted with tiers, usage rates, example bills and free-tier guardrails. Edit it directly or comment, and I'll revise.

One finding worth a look: a task that runs every 5 minutes keeps its workspace awake all month with the default 10-minute idle timeout. That nearly triples the monitor example's bill, from about $28 to about $44.

For implementation, I'd start with the riskiest part: a Fly machine with the Rust agent dialling out to a relay, and a working terminal in a SvelteKit page. To build it here I need:
- **A GitHub repository:** an existing one, or tell me to set up a new structure you'll push yourself.
- **A Fly.io account and API token** when we get to deploying. The first pieces can run locally without it.


---

## User

addressed open questions


---

## Claude

I'll check Fly's terms on reselling with a markup, since you asked me to confirm that.

I've recorded your decisions in the pricing doc. The one you asked me to confirm came back mixed, and the details are in that thread.

Fly's written terms allow "internal use" only and forbid using the service "for the benefit of a third party". On September 23, though, Fly support said on their forum that building platforms on Fly is allowed and that they'll clarify the wording. Neither says anything about charging a markup. Before launch, get Fly to confirm in writing that our case is fine, markup included.

For implementation, I still need a GitHub repository, or your go-ahead to set up the project here for you to push.


---

## User

Good idea, I will need to confirm with fly before launch. This makes our abstraction system even more important then, where our service works regardless of the underlying platform. Before implementation, I would like to discuss what different repos/artifacts we will need, and how we will test these incrementally before going live on the domain


---

## Claude

Agreed. Fly's terms make the compute abstraction a real requirement, not just tidiness. Here's how I'd structure the code and the testing.

**Repositories: one main repo, with a couple of exceptions**

I'd put the platform in a single repository, a monorepo. The agent, relay, control plane and web app share one protocol, and most features touch several of them at once. In one repo, a change to the protocol and every side that uses it lands in a single commit and is tested together. For a solo builder that's a big simplification.

```
croncave/
  crates/                Rust (one Cargo workspace)
    protocol/            agent ↔ relay messages and shared types (also generates TypeScript types)
    compute/             the ComputeDriver interface plus drivers: fake, local, fly
    control-plane/       API, scheduler, orchestrator
    relay/
    ai-gateway/
    agent/               static binary that runs inside workspaces
    db/                  Postgres migrations
  web/                   SvelteKit app, including the view component library
  images/                workspace base images (agent + Claude Code + common tools)
  templates/             starter templates
  infra/                 deploy config per environment
  e2e/                   end-to-end tests (Playwright)
  docs/decisions/        short records of decisions like "why Fly"
```

Separate repositories only where something is published for others to use:
- **SDKs** (R2), such as `croncave` on PyPI and npm, so users can see and trust the code they install.
- **Public templates**, possibly later, if you want community contributions.

**What gets built and shipped**

| Artifact | Built from | Notes |
| --- | --- | --- |
| Server image | control-plane, relay, ai-gateway | One binary with a role flag at first. Split into separate services when scale needs it |
| Agent binary | agent | Static and versioned. Updated when a workspace wakes |
| Workspace base image | images | A standard container image, so it runs on any provider |
| Web app | web | SvelteKit build |
| Database migrations | db | Run before each deploy, written so old and new code both work during a rollout |

**The compute abstraction**

The `ComputeDriver` interface is the only place any provider appears: create, start, stop, suspend, snapshot, resize, destroy, status. It starts with three drivers:

1. **Fake:** in memory, for fast tests.
2. **Local:** Docker containers on your machine. They aren't VMs, but they behave the same from the agent's side.
3. **Fly:** the real thing.

One shared test suite runs against every driver. If Fly's answer is ever "no", a new driver has to pass that suite and nothing else changes. Two more rules keep us portable:
- Nothing Fly-specific inside the workspace.
- The control plane runs as an ordinary container that isn't tied to Fly.

**Testing, in four environments**

1. **Local, on your machine.** One command starts Postgres, S3-compatible storage (MinIO), the server, the web app, and workspaces through the local driver. A fake Claude Code tests agent sessions without spending tokens.
2. **CI, on every push:**
   - unit tests
   - protocol tests
   - the driver test suite, on fake and local
   - full-system tests with Playwright: create a workspace, open the terminal, run a task, see it in the timeline, open a preview
   - security tests, which confirm a workspace can't be reached from outside and that the egress rules block what they should
3. **Staging, on real infrastructure** but kept apart from the real domain:
   - a separate Fly organisation
   - its own domain, such as staging.croncave.com, plus a separate preview domain
   - Stripe in test mode
   - a GitHub App for development
   - a low-limit Anthropic key
   - the full test suite nightly against real Fly machines, measuring wake times and costs

   You also use staging for your own work, as the first real user.
4. **Production, on app.croncave.com, invite-only at first.** Everything is separate from staging. Each alpha founder is added to an allowlist, and new features ship behind feature flags. Agent updates go to a few workspaces first, then to all.

**How the build order becomes checkpoints**

Each step has a test that proves it works before moving on:

| Step | Proven when |
| --- | --- |
| 0. Repo and CI | An empty pipeline runs green, with logging and error tracking wired in |
| 1. Agent + relay, locally | A browser terminal works on a local workspace |
| 2. Fly driver | The same test passes on real Fly in staging, and the driver suite passes |
| 3. Sleep, wake and the scheduler | A scheduled run wakes, runs and sleeps again, and wake time is measured |
| 4. Claude Code sessions | A session keeps running after the browser closes, and caps stop it |
| 5. GitHub and change review | A session opens a pull request, and the review screen shows it |
| 6. Home, timeline, email | "Needs you," "running" and "done" reflect real events |
| 7. Previews | An app on localhost opens in the app from the preview domain |
| 8. Hardening | Security and abuse tests pass. Then the alpha goes live on the real domain |

Want me to add this to the architecture doc as a "Repository and delivery" section? Then, once you create an empty GitHub repository (or tell me to lay out the structure here for you to push), I can start step 0.


---

## User

Do I pull changes from the repo to test locally? Also, will you build this using cloud sessions?


---

## Claude

Yes, and yes: this conversation is already running in a cloud workspace, and that's where I'd build.

**How the work would flow**

1. You create an empty GitHub repository and I connect it to this session.
2. I write the code here, run the tests, and push each piece of work to a branch with a pull request describing what changed and how to try it.
3. GitHub Actions (CI) runs the full test suite on every pull request, so you see green or red before reading any code.
4. To try something yourself, you pull that branch locally (`git fetch && git checkout <branch>`) and run the one-command local setup: Postgres, storage, server, web app and local workspaces. When you're happy, you merge.

Closing your laptop doesn't stop me, and you review in batches. It's the workflow Croncave itself is meant to offer.

**What this workspace can and can't do**

- **Can:** Rust, Node and pnpm are installed, along with a Postgres client, so I can build and unit-test the Rust crates and the SvelteKit app here.
- **Can't run Docker containers.** The Docker command is present, but no Docker service is running, so the local driver and full-system tests likely won't run here. I'll try starting it, but I wouldn't count on it. Those tests run in CI instead, where Docker works, and on your machine.
- **Can't reach most of the internet.** Network access is limited to package registries and GitHub. That means no calls to Fly's or Anthropic's APIs from here. Anything touching real Fly machines runs from CI or your machine, with your tokens stored as GitHub secrets, never pasted into this chat.
- **Is modest in size:** 2 CPUs and 7 GB of memory. Fine for development, though full Rust builds will be slowish.
- **Doesn't last forever.** It's cleaned up after a period of inactivity, so the repository is the source of truth. Nothing important lives only here.

**What I need from you**

1. An empty GitHub repository. Give me the owner/name, and I'll connect it with push access.
2. Later, around step 2, a Fly.io staging organisation and API token added as GitHub Actions secrets, and a low-limit Anthropic key for staging.

Once the repository is connected, I'll start step 0: the repository layout, both workspaces (Rust and SvelteKit), CI, the local one-command setup, and the fake compute driver.


---

## User

I would actually like to develop locally at the start, at least up until R1. I want close visibility and input into everything that happens. After that, I will expand to cloud sessions


---

## Claude

That works well, and it suits the "close visibility" goal. There are two ways to do it.

1. **Link this conversation to your computer.** Open this chat in the Claude desktop app on your machine and link it. I can then work directly in your local repository: read and edit files, run builds and tests, start the local stack. Everything happens in your folder, where you can see it. This session isn't linked right now, so this is the step that makes that possible.
2. **Run Claude Code locally in the repository.** You'd work in your own terminal or editor with a fresh Claude Code session. It won't have this conversation's context, so it needs a written brief to start from.

Either way, I'd suggest a rhythm that keeps you in control:
- **Plan before code.** Each step starts with a short plan: what I'll build, which files change, and how we'll test it. You approve or adjust it before anything is written.
- **Small steps.** Each piece is small enough to read in one sitting, ending with a test you can run yourself.
- **Decisions get recorded.** Anything we decide along the way goes into `docs/decisions/`, so the reasoning isn't lost when you later move to cloud sessions.

One thing I can do right now either way: write a **project brief**, a `CLAUDE.md` file for the repository root. It would summarise the product, the architecture decisions, the repository layout, the coding conventions and the build order, with links to the three docs. Any future session, local or cloud, starts with full context instead of re-deriving it. I'd pair it with the step 0 plan for you to review before any code exists.

Which setup do you prefer? And shall I write the brief now?


---

## User

Let's do it this way. You set up 0. the repo and CI, but don't add any code. However, add context to the repo for my local claude to pick up. This includes links to our artifacts, claude.md and/or agent.md files, and a link to this chat or transcript if possible.
Once all that looks good, I take it from there locally

Let's do it this way. You set up the repo and CI, but don't add any code. However, add context to the repo for my local claude to pick up. This includes links to our artifacts, claude.md and/or agent.md files, and a link to this chat or transcript if possible.
Once all that looks good, I take it from there locally.

Note that I want you to do this setup on github, then I will clone the repo. https://github.com/Croncrave/croncrave

Let's do it this way. You set up the repo and CI, but don't add any code. However, add context to the repo for my local claude to pick up. This includes links to our artifacts, claude.md and/or agent.md files, and a link to this chat or transcript if possible.
Once all that looks good, I take it from there locally.

Note that I want you to do this setup on github, then I will clone the repo. https://github.com/Croncave/croncave


---

## Claude

I'll connect the repository, then set it up with CI and the context files for your local Claude.
