# Security Policy

## Supported versions

Chrysoberyl is an early project. A security fix goes to the default branch.
Older versions are not maintained.

## Reporting a vulnerability

Report a security problem privately. Do not open a public issue.

- Preferred: open a private security advisory with the "Report a
  vulnerability" button on the Security tab of the repository.
- Or send an email to stivenagostingjekaj@gmail.com.

Include the steps to reproduce the problem, the commit, and the effect as you
understand it. You get a first reply in a few days. When a fix is ready, it is
released, and your report is credited unless you ask to stay anonymous.

## Scope

Chrysoberyl reads files that can come from anywhere: images, videos, PDF
documents, SVG files, meshes, rule files and hint files. Each decoder has a
limit on the file size and on the memory that it uses, and a malformed file
must fail with an error. These are in scope:

- A file that makes a decoder use memory past its limit, or that makes it
  crash or hang.
- A rule file or a hint file that makes Chrysoberyl read a file outside the
  directory of that rule file.
- A verdict that a crafted input can change without a change in its pixels.

These are out of scope:

- The GPU drivers of your machine. The GPU path only shows a pair. It never
  decides a verdict.
- A problem that needs write access to your repository or your baseline
  store already.
