# Croncave: Product Definition

> **Snapshot.** Exported from the living doc "Croncave: Product Definition" on 2026-09-29. The source of truth is the doc itself: https://claude.ai/code/artifact/239e25ad-38d2-4426-b307-9d6c29ed40d2 (private to the account owner). If this file and the doc disagree, the doc wins; refresh this snapshot when the doc changes.

Sep 28, 2026 · @Treasure

Croncave gives anyone a private computer in the cloud that works while they're away, managed entirely through a web app instead of a desktop. This doc defines it in two parts: an Amazon-style **PR/FAQ** for the value proposition, and a **User Story Map** of every feature, labelled with **MoSCoW** priorities and sliced into releases.

- **Croncave** is the working name and can change later. The app will live at app.croncave.com.
- **MoSCoW labels are judged against the first public launch.** Must = launch can't happen without it. Should = launch if at all possible. Could = welcome after launch. Won't = deliberately not now, but the design must leave room for it.
- **Releases:** R1 is a private alpha, R2 is the public launch, R3 is the first expansion, and Later is everything after.
- Everything here comes from decisions made so far. Anything not yet decided is listed under Open decisions rather than guessed.

How it's built is covered in the companion doc, [Croncave: Technical Architecture](https://claude.ai/code/artifact/33224792-3002-4d1a-b0d4-a1d9686c2315).

## Press release

*Written as if the public launch (R2) has happened. Quotes are hypothetical, as the PR/FAQ format intends.*

### Croncave launches: a computer in the cloud that keeps working after you close your laptop

**Hand off coding sessions, scheduled jobs and monitors to a private workspace that runs while you sleep, and see the results in plain language when you wake up.**

**\[CITY\], \[LAUNCH DATE\].** Today Croncave opens to everyone. It gives people a private workspace in the cloud where their AI agents, scripts and monitors keep running when their own computer is closed. There is no server to rent, no terminal to learn and no desktop to manage. You describe what you want or bring your own code, and Croncave tells you what happened, what it found and what needs your decision.

**The problem.** More people than ever can start useful work on a computer. They vibe-code their app with an AI agent, write a scraper, or build a spreadsheet of leads to enrich. But that work only runs while their laptop is open and awake. Moving it to the cloud means choosing a provider, sizing a server, installing tools, managing keys and checking logs. Even experienced engineers put that off, and everyone else never starts.

**The solution.** Croncave organises work into **workspaces**, each a private computer that sleeps when idle and wakes when there is work to do. In a workspace you can:

- **Hand an AI agent a job and walk away.** It works overnight with your own AI provider key, pushes to a branch on your GitHub, and leaves a plain-language review: what changed, why, proof that it works, and the code.
- **Run anything on a schedule.** Bring your own script or ask AI to write one. Set when it runs and where the results go, and it runs reliably without your computer.
- **Use what it built.** Apps and sites running inside the workspace open privately in Croncave, as if on your own localhost. No one else can reach them.
- **See it at a glance.** Every workspace gets a clean dashboard built from standard components, plus a timeline of what happened, in plain words.

Every workspace can be set up by chatting with AI or by filling in a simple form, and both produce the same thing. You can always see and edit what the AI did, and you can switch AI off entirely.

**Private by design.** Workspaces never accept incoming connections. They have no public address and no open ports, and by default your AI key is never stored inside them. Spending limits are set before anything runs.

> "I built Croncave because I kept having to stop working when I got tired. Now the work carries on while I sleep, and I only step in to make decisions." — \[FOUNDER NAME\], founder of Croncave

> "I describe the changes our users asked for before bed. In the morning there's a pull request with screenshots and passing tests, and I just decide what to merge." — \[HYPOTHETICAL CUSTOMER\], founder of a real estate app

**Getting started.** Sign in at \[URL\], connect GitHub and an AI key if you want AI features, and start your first workspace in under five minutes. Pricing is based on the time a workspace is awake plus storage, with a spending limit you choose. \[PRICING TBD]

## Customer FAQ

**What exactly is a workspace?** A private computer in the cloud that holds your files, tools and running work. You never see a desktop. You see what the work produced: a dashboard, a timeline, a code review, or a preview of your app. A workspace can run one task or several related ones.

**Do I need to know how to code?** No. You can describe what you want and approve the plan AI proposes, or pick a template and fill in a form. Engineers can bring their own code, use a terminal, or keep settings in a file in their repo.

**Do I have to use AI?** No. Every workspace has an AI-off switch. With it off, nothing from that workspace is ever sent to an AI provider.

**Which AI providers work, and who pays for them?** At launch, AI sessions run Claude Code. You either connect your own Anthropic API key or sign in with your own Claude subscription, and Anthropic bills you directly. With an API key, Croncave shows what each session cost and stops at the limit you set, and by default the key is never stored inside a workspace. More agents and providers will follow.

**What happens when I close my laptop?** Nothing changes. Work runs in the workspace, not on your computer. When it needs you, you get a notification. Reopen Croncave on any device to pick up where things stand.

**Can anyone else reach my workspace?** No. Workspaces accept no incoming connections and have no public address. Apps running inside one open only for you, through Croncave.

**Can I host my website here?** Not publicly. You can run and use your site or app privately, exactly as you would on localhost. When it's ready, push the code to GitHub and host it wherever you like.

**How do I know what it did?** Each workspace has a timeline in plain language ("Ran at 8:00, found 3 new listings, emailed you"). Coding sessions end in a change review with a summary, screenshots, test results and the code. Raw logs are one level deeper for anyone who wants them.

**What does it cost?** You pay for the time a workspace is awake, plus storage. Workspaces sleep when idle, so a daily job costs minutes, not a month of server time. AI usage is billed by your AI provider. \[PRICES TBD]

**What can't it do?** It can't run desktop apps like iMessage or Photos, and it doesn't host public websites or accept incoming requests. It isn't built for connecting hundreds of apps together the way Zapier is.

**Can my co-founder use the same workspace?** Not at launch. Shared workspaces are planned, and the product is being designed for them from day one.

**Where is it available?** The United States only at launch. Other regions will follow based on demand.

## Internal FAQ

**What are the product principles every feature must respect?**

1. **The work is the product, not the machine.** Users see workspaces and results. Machines, VMs and packing appear only in the details layer.
2. **Nothing connects in.** Workspaces only make outgoing connections. Previews, terminals, logs and triggers all travel through the platform over the workspace's own outgoing connection.
3. **AI and manual are one system.** Every workspace and task is defined by the same five answers: when it runs, what it does, what it remembers, where results go, and its limits. AI fills in the same form a person would.
4. **Standard views, not generated UI.** Dashboards are composed from a fixed component library. AI chooses and connects components. Manual code feeds them through the SDK.
5. **Layered disclosure.** Each screen has a default layer, a details layer and an expert layer, in the Apple pattern.
6. **AI proposes, people approve.** Changes that matter arrive as a proposal to accept, edit or reject: change reviews, plan cards, notes with statuses.
7. **Sleep by default.** A workspace is awake only while work runs or someone is looking at it.
8. **Team-ready data model from day one.** Workspaces belong to an owner, which can be a person or a team. Every action is attributed to someone, and work happens in sessions.

**Who is the first customer?** Technical founders and indie builders who already use coding agents and have scripts they want off their laptop. They feel the pain most, tolerate rough edges, and exercise the core. Non-technical users are the goal, and they arrive through templates and AI setup in R2 and R3. \[CONFIRM]

**Why not compete on cloud coding agents alone?** The AI labs and editor makers already bundle cloud coding agents into existing plans: Claude Code on the web, Codex cloud, Cursor, Jules and GitHub Copilot ([aq.dev](https://aq.dev/guides/best-cloud-coding-agents/)). Our angle is any agent, next to every other workload, with a better review, sleep-based pricing and a firm privacy model.

**Who is the closest competitor, and how are we different?** Manus Cloud Computer, launched April 30, 2026, is an always-on Ubuntu machine driven by plain-English instructions ([Eyerys](https://www.eyerys.com/articles/news/cloud-computer-use-manus-wants-users-have-agents-run-247-background)). It has not published prices or compliance statements ([AI Automation Global](https://aiautomationglobal.com/blog/manus-cloud-computer-always-on-ai-automation-2026)). We differ on:

- workspaces that sleep, instead of one always-on machine
- standard views
- bring-your-own key for any provider
- the AI-off guarantee
- no incoming connections
- transparent limits

**How does it work technically?**

- **Control plane:** Rust, using Tokio and axum, with Postgres.
- **Workspaces:** Firecracker microVMs with snapshots, so they wake fast. Several of one user's workspaces may share a VM in isolated containers. Different users never share a VM.
- **Agent inside each workspace:** a small Rust binary that opens the outgoing connection and handles tasks, logs, the terminal, files and preview traffic.
- **AI key proxy:** agent API calls go through the platform, which adds the key, meters usage and enforces caps.

R1 rents Firecracker VMs from Fly Machines, and we later move to our own Firecracker system on a cloud provider. The details are in the technical architecture doc.

**How do private previews work?** The workspace's localhost ports travel over its outgoing connection to an authenticated platform proxy. Each preview gets its own isolated web address. Websockets, live reload and cookies must behave as they would on localhost. Only the owner can open a preview, and sharing with signed-in users comes later.

**What are the biggest risks?**

- **Abuse:** crypto mining, scraping sites that forbid it, running bots at scale. Mitigations are egress controls, per-account limits, verified accounts and clear terms.
- **Cost of always-awake work:** bots and long agent sessions keep workspaces awake. It must be shown upfront ("keeps this workspace awake, about $X a month").
- **Agent quality:** a bad overnight session damages trust. Agents work on branches, pull requests are the default, and nothing auto-merges unless tests pass and the user opted in.
- **Bring-your-own-key economics:** paying API rates can cost more than flat subscriptions that include a cloud agent. Supporting subscription logins inside the terminal needs a check of each provider's terms.
- **Money-moving automations** (trading, payments): out of scope until built-in limits, confirmations and legal review exist.

**What are we deliberately not?** Not a public host or server platform. Not a remote desktop. Not an app-integration hub. Not a reseller of Claude Code usage.

## Story map: backbone

The backbone is the journey every user takes, whether they use AI or not. Every feature in the next section sits under one of these activities.

[Diagram in the source doc: story map backbone · 6 activities, 1 loop]

Start and Set up happen once per workspace. Run, Watch, Review and Adjust repeat until the work is done. Manage and, later, Collaborate apply at every step.

## Story map: every feature

Each row is one thing a user can do, written from their side. Priority is judged against the public launch (R2). Release says when it ships. Both are dropdowns, so re-prioritise in place.

### Start

| Story: I can… | Priority | Release |
| --- | --- | --- |
| Describe what I want in one box, then approve or edit the plan AI proposes | Must | R1 |
| Start a blank workspace and set it up by hand | Must | R1 |
| Bring my code from a GitHub repo | Must | R1 |
| Upload a folder or paste a script | Must | R1 |
| Connect an uploaded folder or script to a GitHub repo | Must | R2 |
| Pick a template from a gallery: page monitor, scheduled report, file batch, bots | Must | R1 |
| Have my runtime detected from requirements.txt, package.json or a Dockerfile | Should | R2 |
| Create a single task without thinking about workspaces (it gets a hidden one) | Must | R1 |
| Duplicate an existing workspace | Could | R3 |
| Start from a settings file kept in my repo | Could | R3 |

### Set up

| Story: I can… | Priority | Release |
| --- | --- | --- |
| Choose when it runs: by hand, on a schedule, or keep going until done | Must | R1 |
| Pick resources from plain presets, with exact specs in details | Must | R1 |
| Set limits: max runtime, spending cap, retry on failure | Must | R1 |
| Choose where results go: in-app and email | Must | R1 |
| Add secrets that reach my code as environment variables | Must | R1 |
| Connect GitHub with access to only the repos I pick | Must | R1 |
| Connect my AI provider key, kept outside the workspace | Must | R1 |
| Give a task memory between runs, like the last price it saw | Must | R2 |
| Turn AI off for a workspace, guaranteeing nothing goes to a provider | Must | R1 |
| Write “when this, do that” rules without code | Should | R2 |
| Get results by text message and push notification | Should | R2 |
| Do a dry run that shows what would have happened before going live | Should | R2 |
| Choose installed packages and the base image (expert layer) | Should | R2 |
| Trigger work from a webhook the platform receives | Could | R3 |
| Trigger work from something the platform checks for me, like new email or a changed page | Could | R3 |

### Run

| Story: I can… | Priority | Release |
| --- | --- | --- |
| Rely on the workspace sleeping when idle and waking for work, with files kept | Must | R1 |
| Hand an AI agent a job that continues after I close my laptop | Must | R1 |
| Have the agent push to a branch and open a pull request | Must | R1 |
| Have the agent stop and ask me when it needs a decision | Must | R1 |
| Have scheduled runs wake the workspace, run and let it sleep | Must | R1 |
| Press Run now | Must | R1 |
| Run several tasks in one workspace, with a “don't overlap” option | Must | R1 |
| Choose which agent to use, such as Claude Code or Codex (R1 ships one) | Must | R2 |
| Have a long session resume after a crash instead of restarting | Should | R3 |
| Keep a bot or long process awake, with its monthly cost shown upfront | Should | R3 |
| Opt in to auto-merge when tests pass | Could | R3 |
| Have light checks run without waking the workspace | Could | Later |
| Use GPU workspaces | Won't | Later |
| Let automations move money, such as trades or payments | Won't | Later |

### Watch

| Story: I can… | Priority | Release |
| --- | --- | --- |
| See a home screen of what needs me, what's running and what finished | Must | R1 |
| Read a plain-language timeline of what each workspace did | Must | R1 |
| Open raw logs and exit codes in details | Must | R1 |
| Watch an agent session live, from any device | Must | R2 |
| Get notified when something needs me or fails | Must | R1 |
| See awake time and AI cost per workspace | Must | R1 |
| See a dashboard for each workspace, built by AI from standard components | Must | R1 |
| Feed those components from my own code through the SDK | Must | R2 |
| Use the whole product comfortably in my phone's browser | Should | R2 |
| Get told when something broke because the world changed, with an offer to fix it | Could | R3 |

### Review and use

| Story: I can… | Priority | Release |
| --- | --- | --- |
| Read a change review: each change, why it was made, and its code | Must | R1 |
| Merge or request changes, with the pull request one click away | Must | R1 |
| Download results and upload inputs in a simple file list | Must | R1 |
| Use apps running in the workspace privately, as on localhost | Must | R1 |
| Keep or leave out individual changes before merging | Should | R2 |
| See before-and-after screenshots and proof the change was tested in a browser | Must | R1 |
| Browse the code read-only | Must | R1 |
| Switch a preview between phone, tablet and desktop sizes | Could | R3 |
| Pin notes on a preview for AI to work through | Could | R3 |
| Keep versions of a prototype and compare them | Could | R3 |
| Export a walkthrough video, screenshots or the code | Could | R3 |
| Share a preview with specific signed-in people | Won't | Later |

### Adjust

| Story: I can… | Priority | Release |
| --- | --- | --- |
| Ask for a change in chat and approve the proposed change | Must | R1 |
| Make the same change in the form instead | Must | R1 |
| Pause, resume or delete a task or workspace | Must | R1 |
| Retry a failed run | Must | R1 |
| Open a terminal in the browser (expert layer) | Should | R2 |
| Take over an AI-built task by hand, or ask AI to extend a manual one | Must | R1 |
| Rearrange or edit a workspace's dashboard | Should | R2 |
| Use a command-line tool to deploy, run and read logs | Could | R3 |
| Keep settings in a repo file that syncs both ways with the form | Could | R3 |

### Manage

| Story: I can… | Priority | Release |
| --- | --- | --- |
| Sign up and sign in with email, GitHub or Google | Must | R1 |
| Manage my connections, AI keys and secrets in one place | Must | R1 |
| Set an account-wide spending cap and see usage | Must | R1 |
| Pay for a plan and see invoices (the R1 alpha is free) | Must | R2 |
| Export or delete all my data | Must | R2 |
| Turn AI off for my whole account | Should | R2 |
| See which person or AI made every change | Should | R2 |
| Choose the region my workspaces run in | Could | Later |

### Collaborate

| Story: I can… | Priority | Release |
| --- | --- | --- |
| Own workspaces as a team, even a team of one (data model only, no interface) | Must | R1 |
| Invite people to a workspace with roles | Should | Later |
| See that someone has a session in progress, then watch, queue after it or run in parallel | Should | Later |
| Get warned before overwriting a setting someone just changed | Should | Later |
| Have questions go to whoever started the work | Should | Later |
| Require a second person to approve risky actions, like merging to main | Should | Later |
| See who else is viewing a workspace | Should | Later |
| Choose between personal and shared AI keys | Should | Later |

## Release slices

R1 is the thinnest version that proves the core promise: hand off work, close the laptop, and wake up to a result you can trust. Each later release has to pass a gate before it starts.

[Diagram in the source doc: release slices · 4 phases, 3 gates]

R1 serves the founder scenario and people with scripts. Private previews, dashboards and templates moved into R1, so the designer and the data cruncher are served from the start too. R2 adds deeper control and paid plans.

**Won't have for now.** These are ruled out deliberately, and the design must not paint us into a corner on any of them:

- Public hosting of sites, or accepting incoming requests
- A remote desktop
- An app-integration hub in the style of Zapier
- Reselling users' Claude Code usage, which Anthropic's terms forbid. Built-in AI features use our own Claude API account, billed at cost
- AI models from non-US labs
- GPU workspaces
- Automations that move money, such as trades or payments

## Decisions

Eight of the ten open questions are settled. Compute is now decided too, and pricing has a direction, with details still to come. Items marked \[proposed\] are defaults to confirm.

- [ ] **Name.** Decided: Croncave for now (nightshift.com was taken). The app lives at app.croncave.com, and domains are being registered.
- [x] **First customer.** Technical founders; three are already lined up. R1 still includes some non-technical features (the R1 rows in the story map).
- [x] **R1 agent.** Claude Code, installed unmodified. Setup should be a single guided step.
- [ ] **Compute.** Decided: Fly Machines for R1, then our own system. See the technical architecture doc.
- [ ] **Pricing model.** Direction: three tiers (free, paid, pro). Each has a base subscription plus usage. AI usage is billed at exactly our providers' cost, with no markup. Other usage (compute, storage and so on) is billed at provider cost plus a small percentage, where the provider's terms allow it. Tiers differ in limits and features. Details to follow.
- [x] **AI subscriptions.** Allowed, with conditions from Anthropic's [Claude Code legal and compliance terms](https://code.claude.com/docs/en/legal-and-compliance):
  - Users sign in to the unmodified Claude Code with their own subscription, and sign-in completes on Anthropic's own page.
  - We accept Anthropic's Commercial Terms to host Claude Code.
  - We never collect, store or relay Claude credentials, never offer our own "Sign in with Claude" button, and never pay for or resell usage.
  - We may say we run Claude Code, but can't use its name or logo in our product name or branding.
  - Tradeoff: subscription credentials live inside the workspace, unlike API keys, which stay in our proxy. Subscription limits also assume ordinary individual use.
- [x] **Retention.** Defaults that users can change \[proposed\]:
  - Activity timeline: 1 year
  - Run logs: 30 days
  - Agent session transcripts: 90 days
  - Files: until the user deletes them
  - Deleted workspaces: purged after 30 days
- [x] **Gate targets.** A gate is the evidence required before starting the next release, so we don't build R2 on an R1 that isn't working. R1→R2 \[proposed\]: all three alpha founders use it at least weekly for four straight weeks, with no lost work and no surprise bills. R2→R3 is set at the end of R1.
- [x] **Launch templates.** Iterate on 10–20 templates, bots included.
- [x] **Regions.** US only at launch: workspaces run in US data centers and sign-up is limited to the US. Expand to other regions based on demand, after reviewing each region's privacy laws.
