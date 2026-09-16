# 01 · The interview

Two passes. Pass 1 gets material uncontaminated by the agent. Pass 2 attacks
what Pass 1 produced. They are never run together ([P4](/00-PRINCIPLES)).

```st hidden
@import "./reader.st"
```

Everything here is project-agnostic — `[product]`, `[the thing they said]`,
`[situation]` are slots the agent fills from the live conversation.


## How to run it at all

**Record verbatim, always.** Before any paraphrase, capture the human's exact
words into `stated`. Paraphrase lives beside the original, never over it
([P3](/00-PRINCIPLES)).

**Follow energy, not the script.** The banks below are a *reservoir*, not a
queue. When the human's voice changes — faster, more specific, more annoyed,
suddenly telling a story — that is the signal. Abandon the plan and go there.
Moesta's heuristic ([S10](/08-SOURCES)) is that most of an interview is noise
and a small part is energy; the energy spike is where the real structure sits.

**Silence is a technique.** After an answer that trails off, wait. The
second thing a person says is usually more specific than the first.

**One question at a time.** Stacked questions ("what happened, and how did that
feel, and what would you change?") let the human answer only the easiest one.

**Never ask about the future or about opinions** ([S09](/08-SOURCES)) — those
cost nothing to say and predict nothing. Convert on the fly:

- ✗ *"Would you use a tool that…?"* → ✓ **"Tell me about the last time this broke. What happened next?"**
- ✗ *"Do you like this positioning?"* → ✓ **"What did you first think this was for?"**
- ✗ *"Would people pay for this?"* → ✓ **"What are they spending now — money, hours, delay, risk?"**
- ✗ *"What features matter most?"* → ✓ **"What have you tried, and what did you do by hand afterwards?"**
- ✗ *"Who is your target customer?"* → ✓ **"Describe the last person you watched need this. Where were they?"**


## Pass 1 · Clean

The agent's vocabulary stays out. Questions are assembled from the human's own
words. The agent's hypotheses stay unstated — writing them down privately is
fine and useful; saying them is contamination.

**Success signal:** the human says something the agent could not have written.
**Failure signal:** the human keeps agreeing ([P10](/00-PRINCIPLES)).

### 1.0 — Open

Grove's opening question, unmodified. It presupposes nothing about the product,
the market, or the problem:

> **And what would you like to have happen?**

Then let it run. Do not steer for several minutes.

### 1.1 — The clean twelve

The engine of Pass 1. Substitute the human's *exact* words for `X` and `Y` —
no synonyms, no tidying, no upgrading their word to a better one
([S05](/08-SOURCES)).

**Developing** — hold time still, deepen the current perception:

- And what kind of **X** is that **X**?
- And is there anything else about **X**?
- And where / whereabouts is **X**?
- And is there a relationship between **X** and **Y**?
- And when **X**, what happens to **Y**?
- And that's **X** like what?  ← *the metaphor question; the single highest-yield line in the bank*

**Moving time:**

- And what happens just before **X**?
- And then what happens? / And what happens next?
- And where could **X** come from?

**Intention:**

- And what would **X** like to have happen?
- And what needs to happen for **X**?
- And can **X**?

> **Worked example.** Human says: *"managers are drowning in group chats."*
> Clean follow-ups: *"And what kind of drowning is that drowning?"* ·
> *"And whereabouts is that drowning?"* · *"And that's drowning like what?"* ·
> *"And what happens just before drowning?"*
>
> ✗ Contaminating follow-ups: *"So it's an information overload problem?"* ·
> *"Is that a workflow visibility issue?"* — both replace the human's live
> metaphor with the agent's dead category, and the metaphor does not come back.

### 1.2 — Events, not reasons

Reconstruct real episodes. The unit is one specific occasion, not a pattern
([S09](/08-SOURCES), [S10](/08-SOURCES)).

- Tell me about the last time you watched **[the thing]** go wrong. Start before it went wrong.
- Where were you? Who else was there? What time of day?
- What did you do immediately after?
- Who found out, and how?
- What did it cost — in money, hours, apology, or trust?
- What did you or someone else do by hand to patch it?
- When did you last see it go *right*? What was different that time? ← *exception question ([S07](/08-SOURCES))*

For an existing decision (a tool they adopted, a process they changed), run the
switch timeline ([S10](/08-SOURCES)) — first thought → passive looking →
active looking → decision → first use:

- When did you first start thinking about changing this?
- What happened right before that thought?
- What did you do next — and then what?
- What else did you look at?
- Why didn't you do it sooner? What made you stop looking for a while?
- What finally tipped it?

### 1.3 — The alternatives inventory

Dunford's inversion ([S15](/08-SOURCES)): positioning starts from what a person
would use if the product vanished. **Push past software.** The real competitors
are usually a spreadsheet, a group chat, one reliable employee's memory, and
doing nothing.

- If **[product]** disappeared tomorrow, what would they do instead? Be specific.
- What are they doing *right now*, before ever hearing of it?
- Who is the person who currently holds this together in their head?
- What breaks when that person is on holiday?
- What have they already tried and abandoned? Why did it not stick?
- What do they refuse to do, even though it would work?

### 1.4 — The verbatim harvest

Language is an asset; harvest it raw ([S16](/08-SOURCES)).

- What words do they use for this when they're annoyed?
- What do they call it that is *not* what the industry calls it?
- What did someone say to you that stuck?
- What have you heard more than once, from different people?
- When you explain what you do at a dinner party, what do you say?
- What sentence makes them nod before you've finished it?

Record every one of these under `stated`, verbatim. These are the strongest
candidates for headline slots — a phrase a real person said outperforms one an
agent composed, and it is checkable.

### 1.5 — Mechanism

The buyer must be able to picture *how*, not just *what*
([S14](/08-SOURCES) — sophistication).

- Walk me through what actually happens, in order, from trigger to done.
- What does the person on the ground actually do? On what device?
- What does the manager see, and when?
- Where does it hand off to a human, and what makes it wait?
- What does it refuse to do?
- What does it produce that you could show someone afterwards?

### 1.6 — Boundaries

The most trust-generating material in the entire interview, and the part an
agent will never invent ([P5](/00-PRINCIPLES), [P7](/00-PRINCIPLES)).

- Who is this actively wrong for?
- What has to be true of a market before this works at all?
- Where have you seen it not fit?
- What would you tell someone *not* to use it for?
- What does it not do yet that people assume it does?
- What are you not willing to promise?

### 1.7 — Peak

One positively-framed question, to surface capability and identity without
putting the human on the defensive ([S11](/08-SOURCES)):

- Tell me about a time this worked unusually well. What was happening, who made
  it possible, and what conditions allowed it?

### 1.8 — Desired state

- Suppose this works, a year out. What is different on an ordinary Tuesday?
  ← *miracle-question lineage ([S07](/08-SOURCES))*
- What would someone *stop* doing?
- What would you want them to say to a colleague, in their own words?


## Pass 2 · Adversarial

Now the agent brings hypotheses and invites destruction. Do not run this until
Pass 1 is recorded — a hypothesis offered early becomes the thing the human
responds to, and you have interviewed yourself.

Open by stating the shift plainly: *"I'm going to say what I think I heard, and
I want you to break it."*

### 2.1 — Read back and get corrected

Read the `stated` verbatims back, unedited. Then:

- Where did I get it wrong?
- Which of these is the one that actually matters?
- Which of these did you say but don't really believe?
- What did I not ask about that I should have?

### 2.2 — Premortem

Klein's prospective hindsight ([S18](/08-SOURCES)): assuming failure makes
dissent socially cheap, so the human volunteers things they would otherwise
soften.

- It's a year from now. The site launched, and it pulled in exactly the wrong
  people while the right ones bounced in four seconds. What happened?
- What is the earliest sign we'd have seen that this was going wrong?
- Which sentence on this page would make a knowledgeable buyer roll their eyes?
- What would a competitor say about this page to win against it?

### 2.3 — Falsify the frame

For each `inferred` claim the agent formed in Pass 1, stated openly:

- I think **[claim]**. What would have to be true for that to be wrong?
- Who would read that and think "that's not me at all"?
- If I deleted this claim entirely, what would we lose?

### 2.4 — Force the axes

Two diagnostics that determine headline strategy, and that must be *decided*
rather than defaulted ([S14](/08-SOURCES)). They are orthogonal — resolve them
separately.

**Awareness** — what does the visitor already know at first contact?

> unaware → problem-aware → solution-aware → product-aware → most-aware

- Where does this visitor come from, immediately before landing?
- Do they already know they have this problem, or do they think it's just how
  business works?
- What sentence would they already agree with before reading anything?
- What belief has to change before they can buy?

**Sophistication** — how saturated is the category's claim-space?

> be-first → enlarge-claim → introduce-mechanism → enlarge-mechanism → attach-to-identity

- How many others make essentially this promise? Name them.
- Which claim in this category has been repeated until it means nothing?
- What does a buyer assume is marketing noise the moment they read it?
- What can you show that they cannot?

Mis-reading these is the most expensive error available: a plain benefit claim
in a saturated category reads naive, and an identity narrative in a fresh
category buries an easy, believable promise.

### 2.5 — Onlyness

Force the singular claim, then attack it ([S15](/08-SOURCES) Neumeier):

> Our **[offering]** is the only **[category]** that **[benefit]** for
> **[who]** in **[context]**.

- What disqualifies the word "only"?
- Only compared to which alternatives, in which market, this year?
- If a competitor copied the sentence tomorrow, what would still be true only of you?

### 2.6 — Substitution test

Take each drafted headline:

- Could a competitor put their name in this sentence unchanged? → it says nothing.
- Would the opposite sound equally sensible? → it says nothing.

(Both are formalised as gates in [05-COPY](/05-COPY).)


## Closing both passes

- Is there anything else about **[the central thing]**? ← *clean, and it works to the very end*
- What did you expect me to ask that I didn't?
- What should I not have taken at face value?

Then write the record ([02-RECORD](/02-RECORD)) **before** drafting a single
line of copy. The temptation to write headlines while the conversation is warm
is exactly how unrecorded inference gets in.
