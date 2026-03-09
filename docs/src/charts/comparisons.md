🏛️ Design Philosophy: Charton vs. Kuva
While many visualization libraries (such as Kuva) provide a comprehensive gallery of pre-defined plot types, Charton 0.3+ has shifted toward a Grammar-based architecture. The difference is not just technical; it is a fundamental shift in how a user thinks about data visualization.

Architecture & Strategy Comparison
🧠 The Core Distinction
Kuva prioritizes "Out-of-the-Box" Efficiency: By providing a rich set of specialized templates (e.g., bioinformatics plots), it allows users to generate standard, publication-quality figures via the shortest possible path.

Charton prioritizes "Expressive Freedom": It moves away from rigid "plot types" and instead provides a Grammar of Graphics. In Charton, no chart is an isolated entity; every visualization is a logical manifestation of points, lines, and areas within a specific coordinate system, empowering users to 'invent' visualizations that the library authors themselves never even envisioned.

📝 Guidance for Users
If you need to quickly generate standard figures that follow specific domain conventions, Kuva is an exceptionally efficient toolbox.

If you need to build complex, highly customized, or multi-layered visualization systems, the Grammar of Graphics in Charton offers an unrestricted creative space.