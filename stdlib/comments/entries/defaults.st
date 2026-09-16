/// The default comment vocabulary (PLAN-123).
///
/// Six types, chosen because they are the six things people actually say about
/// a page: an observation, a task, a doubt, a defect, a rationale, and a piece
/// of work handed to an agent. A project extends this vocabulary in its own
/// `_prelude.st` — it never edits this file.
///
/// Every type here MUST carry `%label` + `%docs`; the roster is a UI and an
/// agent-facing contract, and an unexplained type is a bad affordance in both.

/// The plain observation — no follow-up implied, nothing owed by anyone.
/// This is the type a bare `//@:` header resolves to, so it must stay
/// obligation-free: `//@: this section is intentionally asymmetric`.
%comment_type note {
  %label "Note"
  %docs "An observation left for whoever reads this next. No follow-up implied."
  %color "#8a8f98"
}

/// A unit of work small enough that anyone can pick it up.
%comment_type todo {
  %label "Todo"
  %docs "A unit of work anyone can pick up. Resolve it when the work is done."
  %field owner string?
  %color "#3d8be8"
  %agent_hint "Do the work described, then set status: resolved with a reply
               naming what changed. If the work is larger than it looks, reply
               with what you found and leave it open."
}

/// A question needs an ANSWER, which is why it is not a todo: resolving it
/// means replying, not editing. Keeping the two apart is what stops a thread
/// from being closed by a change nobody asked for.
%comment_type question {
  %label "Question"
  %docs "Something that needs an answer before work proceeds. Resolve by replying."
  %field asked-of string?
  %color "#b06ee8"
  %agent_hint "Answer in a reply if you can do so from the code. If answering
               requires a decision the author must make, leave it open and say
               what the options are."
}

/// A defect: something that behaves wrongly, as opposed to something merely
/// unfinished. `severity` is optional on purpose — demanding triage metadata
/// at capture time is how bug reports stop getting written.
%comment_type bug {
  %label "Bug"
  %docs "Something behaves wrongly. Describe what happens and what should happen."
  %field severity string?
  %color "#e05252"
  %agent_hint "Reproduce first, then fix. Reply with the root cause, not just
               the diff — and add a test that fails before the fix. If you
               cannot reproduce it, say so and leave it open."
}

/// The reason a thing is the way it is. Rarely resolvable — it is documentation
/// pinned to a place, and it earns its own type so it never reads as a todo.
%comment_type design {
  %label "Design"
  %docs "Rationale for why this is the way it is. Usually stays open as a record."
  %color "#3da88b"
}

/// Work handed explicitly to an agent. `acceptance` is REQUIRED: a task whose
/// completion cannot be checked is a task nobody can honestly close, and
/// agents in particular need an observable finish line rather than a vibe.
%comment_type agent-task {
  %label "Agent task"
  %docs "Work assigned to an agent, with an observable acceptance check."
  %field acceptance string
  %field priority string?
  %color "#e8a13d"
  %agent_hint "Implement the change in place, then RUN the acceptance check
               named in the acceptance field. Reply with the command you ran
               and its result, then set status: resolved. If the check does not
               pass, leave the comment open and reply with what blocked you."
}
