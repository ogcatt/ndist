# Noveldist Website Frontend/Backend

For compiling tailwind for styles:
    npx tailwindcss -i ./input.css -o ./assets/tailwind.css --watch

For running database migrations:
    cargo run --manifest-path migration/Cargo.toml -- up

Use //REMOVE THIS for sections to visit


to resume addition of new stock system: claude --resume 18795309-b8c1-42ad-bdf8-526e8817a7bf


- add a red unfufilled order count on the orders tab on the sidebar in the admin UI so when new orders come in it is seen.
- remove product form and container material from product details dropdown
- change the SKU prefix when creating products from PBX to NDX.
- On the product page, move the Visibility field to a new section called access which should display on the left under the basic info entry. Also remove the unneeded whitespace at the bottom of the basic product info section, i think it is caused by flex grow or something. Add the feature to this new access section to restrict the product to access groups (should allow you to search through groups then select one with a dropdown). This should update the access_groups property on the product. It should also have a slider for "Show limited preview to normal users" which should change the show_private_preview value.
