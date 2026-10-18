-- Exemplo de mod com script (API de mods 0.12, docs/api-mods.md, "Scripts Lua").
-- A cada 500 ticks cai uma "chuva de comida" na superfície: 30 tentativas em células sorteadas
-- pelo gerador do próprio script (não mexe nos sorteios do jogo) e uma mensagem na faixa.

local INTERVALO = 500
local TENTATIVAS = 30

function ao_iniciar(estado)
  registrar("chuva de comida ligada; API " .. API_VERSAO .. ", jogo " .. estado:tipo_jogo())
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
  mostrar_mensagem("Choveu comida! (" .. caidas .. " pedaços)")
  registrar("tick " .. estado:tempo() .. ": " .. caidas .. " pedaços de comida")
end
