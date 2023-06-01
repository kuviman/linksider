{
  inputs = {
    geng.url = "github:geng-engine/geng";
  };
  outputs = { self, geng }: geng.makeFlakeOutputs (system:
    {
      src = ./.;
    });
}
