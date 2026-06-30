# Resource Collection Simulation

Simulation en terminal écrite en Rust : des robots autonomes explorent une carte
générée procéduralement et collectent des ressources. L'interface est faite avec
Ratatui.

## Lancer

```
cargo run
```

La simulation tourne toute seule. Appuyer sur n'importe quelle touche pour quitter.

## Comment ça marche

- La carte est générée avec du bruit de Perlin pour placer les obstacles, puis on
  disperse des ressources dessus.
- Deux types de ressources : énergie (`E`) et cristaux (`C`), avec une quantité
  aléatoire entre 50 et 200.
- Les éclaireurs (`x`) se baladent sur la carte et signalent à la base les
  ressources et obstacles qu'ils croisent. Ils ne collectent rien.
- Les collecteurs (`o`) vont chercher les ressources connues, en ramassent une
  unité à la fois et la rapportent à la base (`#`) pour la stocker.
- La base sert de point de départ, de stockage et de centre de communication :
  c'est elle qui regroupe tout ce que les robots découvrent.

Chaque robot tourne dans son propre thread. Ils ne partagent pas d'état
directement : ils envoient leurs découvertes à la base par un canal `mpsc`, et
lisent la connaissance commune pour se déplacer. Le pathfinding est un simple BFS
qui évite les obstacles déjà connus.

## Légende

```
O  obstacle      E  énergie       x  éclaireur
#  base          C  cristaux      o  collecteur
```

## Organisation du code

- `map.rs` : génération de la carte et état du monde
- `robot.rs` : comportements des éclaireurs et collecteurs
- `base.rs` : thread de la base qui agrège les découvertes
- `knowledge.rs` : connaissance partagée
- `message.rs` : messages échangés entre robots et base
- `path.rs` : pathfinding BFS
- `ui.rs` : rendu Ratatui
- `main.rs` : lance les threads et la boucle d'affichage

Les paramètres (taille de la carte, nombre de robots, quantité de ressources) sont
en constantes en haut de `main.rs`.
