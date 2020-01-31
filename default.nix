{ stdenv, lib, rustPlatform, fetchFromGitHub, ... }:
rustPlatform.buildRustPackage rec {
  pname = "nix-query";
  version = "0.1.2";

  src = fetchFromGitHub {
    owner = "9999years";
    repo = "nix-query";
    rev = "v${version}";

    sha512 =
      "06q52an1my6208zcgplf2gw1xgm7v7x8qcisz4x39ckvxbl0rr1nj15b82qn256j92gc1bbir04v9d5xmmxm5a1rg60yg673i8nss5s";
  };

  cargoSha256 = "1wicg5709s0i8z72xiz3a7z6m8zv8y5g6i9jkdxrahqkw8fpyaj7";

  meta = with lib; {
    description = "A cached Nix package fuzzy-search.";
    license = with licenses; [ agpl3 ];
    maintainers = with maintainers; [ ];
  };
}
