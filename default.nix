{ stdenv, lib, rustPlatform, fetchFromGitHub, ... }:
rustPlatform.buildRustPackage rec {
  pname = "nix-query";
  version = "1.0.2";

  src = fetchFromGitHub {
    owner = "9999years";
    repo = "nix-query";
    rev = "v${version}";
    sha512 =
      "3yin1rmwggvfgy87qfhbndz8828x2g82csxlv41sibxa09nnblhfl5ig5zpln96lm2zjrck03bg1xmw3ihjkkbcix0zmbrd29502wrm";
  };

  cargoSha256 = "1wicg5709s0i8z72xiz3a7z6m8zv8y5g6i9jkdxrahqkw8fpyaj7";

  meta = with lib; {
    description = "A cached Nix package fuzzy-search.";
    license = with licenses; [ agpl3 ];
    maintainers = with maintainers; [ ];
  };
}
