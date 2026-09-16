/// CMS test-bed — exercises every admin widget path + a relation graph.
/// Served via `cargo run -- serve projects/cms-testbed/`; the admin
/// (/__spacetime/admin/) renders PURELY from the site.json/types.json this
/// project generates. Two mutable collections (posts, authors) with a
/// posts→authors relation (single + multi) so the rebuilt admin can be
/// browser-verified against real dev-ws CRUD.

@version 2026-07-26;

@type Author {
    id: string;
    name: string;
    bio: string;
    avatar: url;
    active: boolean;
}

@type Section {
    heading: string;
    body: richtext;
}

@type Post {
    id: string;
    title: string;
    excerpt: string;
    body: richtext;
    status: "draft" | "review" | "published";
    featured: boolean;
    views: number;
    cover: url;
    tags: string[];
    author: id(Author);
    contributors: id(Author)[];
    sections: Section[];
}

@data collection $posts Post[] from "/data/posts.json" via dev-ws {
    initial: [];
}

@data collection $authors Author[] from "/data/authors.json" via dev-ws {
    initial: [];
}

// Design tokens as a typed SINGLETON (PLAN-034 Wave B). Brand is one instance —
// it lives in the code as a typed $-value, edited structurally by the admin's
// Brand pane (EditAst rewrites this literal), NOT scanned from CSS custom props.
// Reference tokens directly as $brand.<field> (no var() indirection).
@type Brand {
  scarlet: color;
  ink: color;
  surface: color;
  radius: length;
  reveal: duration;
}
@data inline $brand Brand : { "scarlet": "#FF0020", "ink": "#0a0a0a", "surface": "#ffffff", "radius": "8px", "reveal": "600ms" };
@cms(Brand) { editable: inline; }

// Derived tokens (FEAT-108): computed from the brand literals. Edit a literal
// (e.g. scarlet) and these re-derive live — a token SYSTEM, not a flat palette.
@data derive $hover  : $brand.scarlet | darken(0.12);
@data derive $muted  : $brand.ink | alpha(0.55);
@data derive $tintbg : mix($brand.surface, $brand.scarlet, 0.06);

// Prove the $-value: the masthead uses brand tokens directly.
<header class="masthead">CMS test-bed</header>
.masthead { background: $brand.scarlet; color: $brand.surface; border-radius: $brand.radius; padding: 24px; }
// Use a derived token so the live page proves re-derivation.
.masthead:hover { background: $hover; }

h1 { "CMS test-bed" }
p { "This page exists so the admin has a real @type/@data graph to render. Open /__spacetime/admin/" }

// A real, compiling scoped motion directive — the admin Motion pane edits THIS.
.masthead {
  @on &.scroll(name: reveal, scope: cover, start: 0, end: 1, scrub: false) {
        easing: ease-out;
        stagger: 30;
        staggerFrom: first;
        opacity: 0 -> 1;
        translateY: 40px -> 0;
      }
}
