/// scenario: cta
/// style: editorial
/// school: eastern-aesthetics
/// description: The full-screen italic close — one serif sentence the size of the page, revealed word by word, and a single magnetic door out.
/// dependencies: none
/// screenshot: (pending)
/// useWhen: The last screen of a portfolio or editorial page, where restraint has been the argument and one enormous sentence is the conclusion.

@import "../_schools.st"

/* The editorial close: serif, italic, enormous. The reveal splits by words so
   the sentence lands the way it reads; the button is magnetic because after
   this much restraint, the one interactive moment should feel physical. */

@template &showcase-cta-editorial($line, $accent, $cta) {
    <section class="showcase-cta-ed">
        <h2 class="showcase-cta-ed__title">`$line` <em>`$accent`</em></h2>
        <span class="showcase-cta-ed__cta">`$cta`</span>
    </section>
}

.showcase-cta-ed {
    --sc-east-surface;
    min-height: 88vh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    text-align: center;
    padding: 0 48px;
}

.showcase-cta-ed__title {
    font-family: Georgia, "Times New Roman", serif;
    font-style: italic;
    font-weight: 400;
    font-size: clamp(3rem, 9vw, 9rem);
    letter-spacing: -0.02em;
    line-height: 1.02;
    margin: 0;
    max-width: 12ch;

    @reveal(split: "words", stagger: 70, duration: 800)
}

.showcase-cta-ed__title em {
    font-style: normal;
    color: #e08a63;
}

.showcase-cta-ed__cta {
    display: inline-block;
    margin-top: 7vh;
    font-size: 14px;
    font-weight: 700;
    letter-spacing: 0.08em;
    padding: 19px 46px;
    border-radius: 999px;

    @magnetic(strength: 0.4, radius: 180, lerp: 0.14)
}
