-- Menu de trapaças para testar o jogo (API de mods 0.17, docs/api-mods.md; LEIAME.md ao lado).
-- F12 abre o menu. Cada item chama uma ação da API, que usa a rotina do jogo original para aquilo
-- (o começo da chuva, _XferPatch, _YellowDeath, o Mystery...). Toda trapaça usada aparece na faixa
-- ("Trapaça: ...") e fica marcada no painel; o jogo salvo não guarda isso (o .ANT não tem campo).

local TECLA = "F12"

-- Itens do menu: texto e função. "-" é separador.
local itens = {}
local acoes = {}

local function item(texto, f)
  itens[#itens + 1] = texto
  acoes[#itens] = f
end

local function separador()
  itens[#itens + 1] = "-"
end

-- Estado do próprio mod (não vai para o save).
local painel_ligado = false
local procurando_evento = false
local usadas = 0
local proxima_casa = 0

local function avisar(texto)
  mostrar_mensagem("Trapaça: " .. texto)
end

-- Célula livre ao lado da amarela (na superfície ou no ninho em que ela está).
local function em_volta(estado, f)
  local a = estado:amarela()
  if not a.plano then
    return 0
  end
  local feitas = 0
  for dx = -1, 1 do
    for dy = -1, 1 do
      if (dx ~= 0 or dy ~= 0) and f(a.plano, a.x + dx, a.y + dy) then
        feitas = feitas + 1
      end
    end
  end
  return feitas
end

local function atualizar_painel(estado)
  if not painel_ligado then
    definir_painel(nil)
    return
  end
  local s, r = estado:sementes()
  local chuva = estado:chuva()
  local m = estado:menino()
  local px, py = estado:patch()
  local p, v = estado:colonia("preta"), estado:colonia("vermelha")
  local licao, dada = estado:licao()
  local a = estado:amarela()
  local linhas = {
    string.format("GameTime %d  Cycle %d  %s", estado:tempo(), estado:ciclo(), estado:tipo_jogo()),
    string.format("S %04X  R %08X", s, r),
    string.format("patch (%d, %d)  amarela %s (%d, %d) vida %d", px, py, a.plano or "-", a.x, a.y, a.saude),
    string.format("pretas %d (saúde %d)  vermelhas %d (saúde %d)", p.adultos, p.saude, v.adultos, v.saude),
    string.format("rainhas guardadas: pretas %d  vermelhas %d", p.rainhas_guardadas, v.rainhas_guardadas),
    string.format("chuva %s (%d)  _WaterTable %d", chuva.chovendo and "sim" or "não", chuva.contagem, chuva.lencol),
    string.format("_FlyAway pretas %d  vermelhas %d", estado:revoada("preta"), estado:revoada("vermelha")),
    string.format("menino: nó %d %s%s", m.no, m.dentro_de_casa and "em casa" or "lá fora",
      m.cortando_grama and ", cortando" or ""),
    string.format("lição %d%s", licao, dada and " (dada)" or ""),
    string.format("trapaças usadas: %d", usadas),
  }
  definir_painel(table.concat(linhas, "\n"))
end

-- Chuva -----------------------------------------------------------------------------------
item("Começar chuva", function(e) e:comecar_chuva() end)
item("Parar chuva", function(e) e:parar_chuva() end)
separador()
-- Menino ----------------------------------------------------------------------------------
item("Menino: cortar a grama neste patch", function(e) e:menino_cortar_grama() end)
item("Menino: inseticida dentro de casa", function(e) e:menino_inseticida() end)
item("Ir para o patch da cozinha (0, 12)", function(e)
  e:povoar_patch("preta", 0, 12, 3)
  e:trocar_patch(0, 12)
end)
item("Ir para o próximo patch da casa", function(e)
  -- Patches da casa: índice x*16+y de 0 a 37 (quintal.md, _MakeMap).
  proxima_casa = (proxima_casa + 1) % 38
  local x, y = proxima_casa // 16, proxima_casa % 16
  e:povoar_patch("preta", x, y, 3)
  e:trocar_patch(x, y)
  return string.format("patch (%d, %d)", x, y)
end)
item("Ir para o patch do quintal ao lado", function(e)
  local px, py = e:patch()
  local x = px < 11 and px + 1 or px - 1
  e:povoar_patch("preta", x, py, 3)
  e:trocar_patch(x, py)
  return string.format("patch (%d, %d)", x, py)
end)
separador()
-- Colônias e revoada ----------------------------------------------------------------------
item("Guardar 3 rainhas pretas (botão PLACE)", function(e) e:guardar_rainhas("preta", 3) end)
item("Revoada vermelha (25 reprodutores)", function(e)
  e:criar_reprodutores("vermelha", 25)
  e:armar_revoada("vermelha")
end)
item("Revoada preta (25 reprodutores)", function(e)
  e:criar_reprodutores("preta", 25)
  e:armar_revoada("preta")
end)
separador()
-- Bichos ----------------------------------------------------------------------------------
item("Gato aparecer", function(e) e:trazer_gato() end)
item("Cachorro latir", function(e) e:latir() end)
item("Aranha aparecer", function(e) e:trazer_aranha() end)
item("Formiga-leão perto da amarela", function(e)
  local a = e:amarela()
  if a.plano ~= "superficie" then
    return "a amarela não está na superfície"
  end
  e:por_formiga_leao(math.min(math.max(a.x + 4, 1), 126), math.min(math.max(a.y, 1), 62))
end)
separador()
-- Amarela ---------------------------------------------------------------------------------
item("Luta: soldado vermelho junto da amarela", function(e)
  local a = e:amarela()
  if a.plano ~= "superficie" then
    return "a amarela não está na superfície"
  end
  local n = em_volta(e, function(_, x, y) return e:colocar_formiga("vermelha", x, y, "soldado") end)
  return n > 0 and (n .. " soldados") or "sem lugar livre"
end)
item("Comida em volta da amarela", function(e)
  local n = em_volta(e, function(plano, x, y) return e:colocar_comida(plano, x, y) end)
  return n .. " pedaços"
end)
item("Curar a amarela", function(e) e:saude_amarela(100) end)
item("Matar a amarela", function(e) e:matar_amarela() end)
separador()
-- Tempo -----------------------------------------------------------------------------------
item("Avançar 100 ticks", function() avancar_ticks(100) end)
item("Avançar 500 ticks", function() avancar_ticks(500) end)
item("Avançar 1000 ticks", function() avancar_ticks(1000) end)
item("Pular até o próximo evento", function()
  procurando_evento = true
  avancar_ticks(5000)
end)
separador()
-- Jogo ------------------------------------------------------------------------------------
item("Tutorial: avançar lição", function(e) e:avancar_licao() end)
item("Fim de jogo: vitória", function(e) e:terminar_jogo("vitoria") end)
item("Fim de jogo: derrota", function(e) e:terminar_jogo("derrota") end)
item("Mystery", function(e) e:misterio() end)
item("Painel de depuração (liga/desliga)", function(e)
  painel_ligado = not painel_ligado
  atualizar_painel(e)
  return painel_ligado and "ligado" or "desligado"
end)

-- Ganchos ---------------------------------------------------------------------------------

function ao_iniciar(estado)
  definir_menu({ titulo = "Trapaças", tecla = TECLA, itens = itens })
  registrar("menu de trapaças em " .. TECLA .. " (API " .. API_VERSAO .. ")")
end

function ao_menu(indice, texto, estado)
  local f = acoes[indice]
  if not f then
    return
  end
  local detalhe = f(estado)
  usadas = usadas + 1
  avisar(detalhe and (texto .. " (" .. detalhe .. ")") or texto)
  registrar("trapaça: " .. texto .. (detalhe and (" - " .. detalhe) or ""))
  atualizar_painel(estado)
end

function ao_evento(ev, estado)
  if not procurando_evento then
    return
  end
  if ev.tipo == "mensagem" or ev.tipo == "musica" or ev.tipo == "fim_de_jogo" or ev.tipo == "mensagem_faixa" then
    procurando_evento = false
    parar_avanco()
    mostrar_mensagem(string.format("Trapaça: parou no tick %d (%s %s)", estado:tempo(), ev.tipo, tostring(ev.id or ev.indice or "")))
  end
end

function depois_do_tick(estado)
  if painel_ligado then
    atualizar_painel(estado)
  end
end
