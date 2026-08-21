{ zatsu }:
{
  lib,
  config,
  ...
}:
let
  zatsuPrompt = ''
    ### zatsu

    A deterministic repository and file outline viewer. Directory mode shows a .gitignore-aware tree and exported symbol signatures with line numbers.

    Prefer `zatsu` over `cat` when you need repository structure or signatures, not full implementation. Use the line numbers in the output to inspect specific sections.

    Supported languages: C, C++, C#, Go, Haskell, Java, JavaScript, Kotlin, Markdown, Python, Ruby, Rust, Swift, TypeScript/TSX

    `zatsu` exits with code 1 for unsupported languages.
  '';
in
{
  options.programs.zatsu = {
    enable = lib.mkEnableOption "zatsu code outline viewer";
    claude-code = {
      enable = lib.mkEnableOption "claude-code integration for zatsu";
    };
    codex = {
      enable = lib.mkEnableOption "Codex integration for zatsu";
    };
  };

  config = lib.mkIf config.programs.zatsu.enable (
    lib.mkMerge [
      {
        home.packages = [ zatsu ];
      }
      (lib.mkIf config.programs.zatsu.claude-code.enable {
        programs.claude-code.rules.zatsu = zatsuPrompt;
      })
      (lib.mkIf config.programs.zatsu.codex.enable {
        programs.codex.context = zatsuPrompt;
      })
    ]
  );
}
