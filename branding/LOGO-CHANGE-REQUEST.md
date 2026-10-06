# Solicitação de alteração da logo — ZapFast Business

Preservar exatamente a identidade visual original do ZapFast — círculo verde,
gradientes, aro e acabamento — substituindo somente o símbolo central pela
letra `B` escura e centralizada.

## Arquivos que o aplicativo deve consumir

| Uso | Arquivo |
| --- | --- |
| Fonte vetorial principal | `packaging/icons/zapfast.svg` |
| Ícone para tamanhos pequenos | `packaging/icons/zapfast-small.svg` |
| Bandeja monocromática | `packaging/icons/zapfast-tray.svg` |
| Executável e instalador Windows | `packaging/windows/zapfast.ico` |
| Fonte macOS | `packaging/macos/icon-1024.svg` e `.png` |
| Documentação | `docs/assets/images/logo.svg` |

O código também foi atualizado para usar a marca com `B` no watermark interno
e para gerar corretamente a versão monocromática da bandeja.

## Entregáveis prontos

Em `branding/export/` estão disponíveis:

- SVG completo, reduzido e monocromático em `branding/`;
- ICO Windows multirresolução;
- PNGs de 16 a 1024 pixels;
- avatar quadrado para GitHub e X;
- cabeçalho do X em 1500 × 500;
- social preview do GitHub em 1280 × 640.

## Critérios de aceite

1. O executável, o instalador e o desinstalador mostram a nova marca.
2. Atalhos, barra de tarefas, bandeja e notificações mostram a nova marca.
3. Login, bloqueio, Configurações, Sobre e watermark interno mostram a nova marca.
4. O ícone continua legível em 16, 20, 24 e 32 pixels.
5. A letra `B` aparece centralizada no mesmo disco verde da marca original.
6. A identidade de instalação e os dados isolados do ZapFast Business não são
   alterados pela troca visual.
