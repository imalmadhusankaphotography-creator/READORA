# readora-sample.pdf

A three-page test document that `packaging/readora/smoke-test.ps1` opens on every Windows build
(titles, two-column justified text, a table, gradients and Sinhala text, including conjuncts such
as ශ්‍රී and ක්‍රි). Written for READORA and printed to PDF with Chrome. It embeds subsets of
DejaVu Sans/Serif (Bitstream Vera licence) and Noto Sans Sinhala (SIL Open Font License 1.1);
no other fonts or images. Licence: MIT OR Apache-2.0, like the repository.

# sinhala-scan.pdf

The top of that document's third page, rendered at 200 dpi in grey, turned by 0.6° and saved as a
picture-only PDF: a "scanned" Sinhala page with no text layer. The smoke test reads it with
Recognize Text in Sinhala and checks that Sinhala text comes back.
