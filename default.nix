{ stdenv, lib, rustPlatform, fetchFromGitHub, ... }:
rustPlatform.buildRustPackage rec {
  pname = "nix-query";
  version = "0.1.0";

  src = fetchFromGitHub {
    owner = "9999years";
    repo = "nix-query";
    rev = "v${version}";

    sha512 =
      "00i6k9v8a1cidr5wkcyjvm3slv8kf101a8yww1r9m971jas78dbj3whzvrrsyf11i5wjwxhwv6idv5p0a23l293k2ck3baxd7m6nk77";
  };

  cargoSha256 = "1wicg5709s0i8z72xiz3a7z6m8zv8y5g6i9jkdxrahqkw8fpyaj7";

  meta = with lib; {
    description = "A cached Nix package fuzzy-search.";
    license = with licenses; [ agpl3 ];
    maintainers = with maintainers; [ ];
  };
}
