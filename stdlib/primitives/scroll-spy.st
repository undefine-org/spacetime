/// Scroll Spy Primitive
///
/// Tracks which registered section currently occupies the viewport center.
/// Uses IntersectionObserver with rootMargin to create a detection band
/// at the viewport center line. Reports which section is active,
/// scroll progress within the section, and scroll direction.
///
/// @param &el - The section element to track
///
/// @yields $isActive - Whether this section is currently the active one
/// @yields $progress - Scroll progress within section (0-1)
/// @yields $direction - Last scroll direction: "up" or "down"

%primitive scroll-spy(&el) {
  %emit js {
    const section = %&el;
    let lastScrollY = window.scrollY;
    let direction = 'down';

    // Track scroll direction
    const onScroll = () => {
      const newY = window.scrollY;
      if (newY !== lastScrollY) {
        direction = newY > lastScrollY ? 'down' : 'up';
        lastScrollY = newY;
      }
    };
    window.addEventListener('scroll', onScroll, { passive: true });

    // Create observer with center-line detection band
    const observer = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        const active = entry.isIntersecting;
        %yield active -> $isActive;
        %yield direction -> $direction;

        if (active) {
          // Calculate progress: how far through the section
          const rect = entry.boundingClientRect;
          const viewH = window.innerHeight;
          const sectionH = rect.height;
          // progress = how far the section top has scrolled past viewport top
          // 0 = section top at viewport center, 1 = section bottom at viewport center
          const scrolled = viewH / 2 - rect.top;
          const progress = Math.max(0, Math.min(1, scrolled / sectionH));
          %yield progress -> $progress;
        }
      }
    }, {
      // Detection band at viewport center (0.2% height)
      rootMargin: '-49.9% 0px -49.9% 0px',
      threshold: [0, 0.01, 0.99, 1]
    });

    observer.observe(section);

    // Initial values
    %yield false -> $isActive;
    %yield 0 -> $progress;
    %yield 'down' -> $direction;

    %cleanup {
      observer.disconnect();
      window.removeEventListener('scroll', onScroll);
    }
  }

  %exports {
    $isActive: bool     // true when this section is the active one
    $progress: number   // 0.0 (just entered) to 1.0 (about to leave)
    $direction: string  // "up" or "down" — last scroll direction
  }
}
