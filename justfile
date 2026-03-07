package:
  rm -rfv .build/
  makepkg -fi

clean:
  rm -rfv .build/
  cargo clean -vv
