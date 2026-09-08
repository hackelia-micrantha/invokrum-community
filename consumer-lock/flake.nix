{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/c5c4a43b0e8056328ec4529f735cabdb8f1942bb";
    invokrum = {
      url = "github:hackelia-micrantha/invokrum-community/f8626e88978159aba52388af83f63738bdf38c58";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { ... }: { };
}
