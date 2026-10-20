-- Example of a mod with a script (mod API 0.12; 0.43 for the texts; docs/api-mods.md, "Lua scripts").
-- Every 500 ticks a "food rain" falls on the surface: 30 tries on cells drawn by the script's own
-- generator (it does not touch the game's random draws) and a message in the strip.
-- The texts come from data/idiomas/<language>.json inside the mod (keys "mod.exemplo-lua.*"),
-- read with traduzir(); English is the fallback and pt-BR is a second example.

local INTERVALO = 500
local TENTATIVAS = 30

local function t(chave, params)
  return traduzir("mod.exemplo-lua." .. chave, params)
end

function ao_iniciar(estado)
  registrar(t("ligada", { api = API_VERSAO, jogo = estado:tipo_jogo(), idioma = idioma() }))
end

function depois_do_tick(estado)
  if estado:tempo() % INTERVALO ~= 0 then
    return
  end
  local largura, altura = estado:dimensoes("superficie")
  local caidas = 0
  for _ = 1, TENTATIVAS do
    local x = aleatorio(0, largura - 1)
    local y = aleatorio(0, altura - 1)
    if estado:colocar_comida("superficie", x, y) then
      caidas = caidas + 1
    end
  end
  mostrar_mensagem(t("choveu", { n = caidas }))
  registrar(t("registro", { tick = estado:tempo(), n = caidas }))
end
