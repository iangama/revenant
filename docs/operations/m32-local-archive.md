# Revenant — arquivo local M32

Este pacote privado preserva o cliente Windows/Linux, os serviços Docker e
uma cópia das fontes do M32. A versão continua 0.2.0, com Protocolo V2 e
compatibilidade V1. Credenciais, partidas salvas e relatórios pessoais ficam
fora do pacote. Não é uma publicação ou uma nova versão comercial.

## Instalar e jogar

1. Extraia o ZIP inteiro para uma pasta nova. Não misture arquivos de pacotes.
2. No Windows, execute `operator/Test-RevenantArchive.ps1` pelo PowerShell.
   No Linux/WSL, execute `python3 -B operator/verify.py .` na pasta extraída.
   A verificação exige todos os arquivos originais, sem alterações ou extras.
3. No Linux/WSL com Docker Compose, Python 3.11+ e OpenSSL, execute
   `bash operator/stack.sh start`. As imagens Docker estão incluídas;
   iniciar os serviços não exige baixar imagens ou compilar o jogo.
4. No Windows, execute `operator/Start-Revenant.ps1`; no Linux, execute
   `bash operator/start-linux.sh`. O cliente Linux requer um ambiente gráfico
   com OpenGL 3.3. O cliente Windows requer Windows x86_64 e driver compatível.
5. Confira sua identificação local e conecte. Use Configurações para escolher
   português, inglês, tamanho de interface, contraste, legendas, redução de
   movimento/flash e remapeamento de teclado/controle.

Os serviços usam apenas `127.0.0.1`: jogo 7000, saúde 8080 e Inspector 4173.
O PostgreSQL não publica uma porta. O projeto Docker padrão é `infra`, com
o volume de partidas `infra_revenant-postgres`. As credenciais ficam em
`~/.local/state/revenant/m30-secrets/current` no Linux. Para usar o diretório
privado já existente, defina `REVENANT_SECRETS_DIR` antes de iniciar.
Se houver partidas e faltarem as credenciais, a inicialização é recusada;
recupere as credenciais originais, sem gerar substitutas sobre esse volume.

Use `bash operator/stack.sh status` para consultar os serviços e
`bash operator/stack.sh stop` para pará-los preservando os dados. O Inspector
é somente leitura. A observação opcional de playtest fica desativada pelos
iniciadores deste pacote. Não adicione logs, backups ou configurações à pasta
selada; guarde-os separadamente.

## Atualização, falhas e retorno ao pacote anterior

Extraia cada atualização para outra pasta e verifique seus checksums. Preserve
o pacote anterior, o diretório privado de credenciais e o volume PostgreSQL.
Com o jogo fechado, crie um backup privado usando o procedimento existente em
`docs/operations/postgresql-backup.md` nas fontes. O comando M30 de backup
trabalha com o projeto padrão `infra` e prova a restauração em um alvo descartável.
Ele não substitui o banco de trabalho.

Pare os serviços pelo iniciador do pacote anterior e inicie pelo novo, com o
mesmo projeto `infra` e as mesmas credenciais. As migrações existentes são
idempotentes; o M32 não acrescenta migração ou formato de recompensa. Para
voltar entre os pacotes M31/M32 compatíveis, pare os serviços e inicie os do
pacote anterior com esse mesmo volume. Essa compatibilidade não se estende
automaticamente a futuros pacotes com alterações de esquema.

Após queda do cliente, reconecte com a mesma identificação. A incursão ativa
recomeça; itens, experiência e equipamento já confirmados pelo servidor
permanecem salvos. Reinicie os serviços com `stack.sh start` após uma queda
do Docker. Configurações locais usam `revenant-settings.cfg` no diretório de
usuário do Godot e recuperam a cópia `.bak` se o arquivo principal estiver
corrompido. Uma instalação nova reutiliza esse diretório, sem copiar
configurações para dentro do pacote.

Os procedimentos aceitos de backup, restauração, privacidade, monitoramento,
incidentes e rollback estão nas fontes, em `docs/operations` e `docs/security`.
Dados posteriores ao backup não estão incluídos nele. A restauração do banco
de trabalho exige um procedimento específico; os validadores usam alvos
descartáveis e nunca removem o volume de trabalho.

## Identidade e reprodução

`SHA256SUMS` sela todos os arquivos, e o arquivo `.zip.sha256` ao lado do ZIP
sela o arquivo completo. `evidence/BUILD-IDENTITY.json` registra a versão do
Godot, o hash do editor e dos templates, as imagens Docker por ID imutável e
o hash do manifesto das fontes. `source/source.tar.gz` contém as fontes
exatas correspondentes a `evidence/SOURCE-SHA256SUMS`.

Para conferir as fontes, extraia o TAR para outra pasta e execute
`sha256sum --check /caminho/do/pacote/evidence/SOURCE-SHA256SUMS` dentro da
pasta `source` extraída. Para montar outro arquivo local a partir do projeto,
compile as imagens `infra-gateway` e `infra-inspector`, disponibilize o editor
e os templates Godot 4.7.1 e execute
`python3 -B scripts/package-m32-archive.py /pasta/externa/nova`.

A montagem normaliza ordem, datas e permissões do ZIP e compara duas
serializações dos mesmos arquivos. Isso garante um ZIP idêntico para os
mesmos artefatos; não afirma que recompilar Rust, Godot ou Docker em outro
sistema produzirá binários idênticos. Recompilar as fontes pode exigir obter
as ferramentas e dependências externas. Jogar usa os binários já incluídos.

As verificações de acessibilidade usam navegação e controles sintéticos,
inspeção de capturas e execuções locais. Não representam certificação,
avaliação por outros jogadores ou cobertura de controles físicos e
tecnologias assistivas não testadas.
