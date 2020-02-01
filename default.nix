{ stdenv, lib, rustPlatform, fetchFromGitHub, ... }:
rustPlatform.buildRustPackage rec {
  pname = "nix-query";
  version = "1.1.0";

  src = fetchFromGitHub {
    owner = "9999years";
    repo = "nix-query";
    rev = "v${version}";
    sha512 =
      "0pyl7h560g0kk3cf2866pznbghrg0n7n7l3ljnjac16chbldwy7whsjnv0ac13gi64k1213fhh3s92mykbwmdr6m3jqf38d83x4h5ni";
  };

  cargoSha256 = "17kwc4ndwd5bkz7lsy2dgi8kphfvfaclx4npc36p06c6sf5f1k8p";

  meta = with lib; {
    description = "A cached Nix package fuzzy-search.";
    license = with licenses; [ agpl3 ];
    maintainers = with maintainers; [ ];
  };
}
