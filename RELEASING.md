# Releasing a port that uses ouro-framework

The ports take `ouro-ui` and `ouro-play` by path, from `../ouro-framework`
next to them. That keeps working on both at once simple (a change here is in
the ports at the next build) and keeps their `Cargo.lock` stable: a path
dependency has no source in the lock file, so it reads the same wherever the
framework comes from.

A build from a port's archive alone (Homebrew, CI) has no
`../ouro-framework`, so it brings the framework itself: the framework's
archive at the tag the port was released with, unpacked into the port's
folder as `ouro-framework/`, and the port's `Cargo.toml` pointed there. With
that, `cargo install --locked` builds as at home (tried 2026-11-20 with
S.C.Out: its `git archive` and the framework's, `--locked`, built and ran).

(A git dependency was tried first and dropped: with a local `[patch]` for
working on both, every local build rewrites `Cargo.lock` between the git
source and the path, and `--locked` fails on the path form.)

## Steps

1. The framework: everything the port needs committed; a tag
   (`git tag v0.1.0`), pushed with the repository
   (`metalsnake-classic-mac-ports/ouro-framework`, public).
2. The port: release as before (CHANGELOG, version, tag); its README or
   CHANGELOG names the framework tag it was built with.
3. The formula in the tap (`homebrew-games`), besides the port's `url`:

   ```ruby
   resource "ouro-framework" do
     url "https://github.com/metalsnake-classic-mac-ports/ouro-framework/archive/refs/tags/v0.1.0.tar.gz"
     sha256 "..." # curl -L <url> | shasum -a 256
   end

   def install
     (buildpath/"ouro-framework").install resource("ouro-framework")
     inreplace "Cargo.toml", "../ouro-framework/", "ouro-framework/"
     system "cargo", "install", *std_cargo_args(path: "crates/scout")
   end
   ```

   (`crates/scout` for S.C.Out, `crates/harry` for Harry, `crates/quagmire`
   for Quagmire.) The `head` build takes the same resource: a head build
   of the port with the framework's tag, so a port's `master` that needs a
   newer framework needs a new framework tag first.
4. A CI of a port checks out both repositories side by side, as at home.

## The version

One version for both crates (`workspace.package.version`), raised with
each tag: 0.x while the screens still change; a change a port must follow
(a field, a function's arguments) raises the minor version.
