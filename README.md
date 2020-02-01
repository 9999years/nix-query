# nix-query

A cached [fuzzy-finder][skim] for the [Nix] package manager.

![A screenshot of nix-query running, displaying a fuzzy-search for gzip and a preview pane displaying information about the package such as its version, license, long description, homepage, and more.][screenshot]

nix-query stores a cache (only package attribute names, about 2MB for
`nixos-stable` and `nixpkgs-unstable` combined) which allows it to instantly
show results (even between terminals), unlike `nix-env --query --available`,
which can take a good bit to finish a query.

When you update your channels / packages, run `nix-query --clear-cache` to
delete the cache file. Automatic cache expiry based on `~/.nix-defexpr`
coming... maybe at some point in the future if I want to? Or if you submit a
pull request?

Uses [skim] for fuzzy-finding.

[skim]: https://github.com/lotabout/skim
[Nix]: https://nixos.org/nix/
[screenshot]: https://raw.githubusercontent.com/9999years/nix-query/master/img/screenshot-gzip.png