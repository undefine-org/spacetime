%macro badge {
    %form  { @badge($label:string) { $styles:properties } }
    %binds { badge-emit($label, styles: $styles) -> {} }
}
%primitive badge-emit {
    %emit css { .badge::before { content: "$label"; } }
}
