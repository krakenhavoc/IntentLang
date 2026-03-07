# Shopping Cart

A shopping cart system with item management, quantity updates, and checkout with inventory validation. Demonstrates collection types, nested entity references, and multiple related actions.

**File:** [`examples/shopping_cart.intent`](https://github.com/krakenhavoc/IntentLang/blob/main/examples/shopping_cart.intent)

```intent
module ShoppingCart

--- A shopping cart system supporting item management,
--- quantity updates, and checkout with inventory validation.

entity Product {
  id: UUID
  name: String
  price: Decimal(precision: 2)
  stock: Int
  status: Available | Discontinued
}

entity CartItem {
  product: Product
  quantity: Int
}

entity Cart {
  id: UUID
  owner: UUID
  items: List<CartItem>
  created_at: DateTime
  checked_out: Bool
}

entity Order {
  id: UUID
  cart: Cart
  total: Decimal(precision: 2)
  status: Pending | Confirmed | Shipped | Delivered | Cancelled
  created_at: DateTime
}

action AddItem {
  --- Add a product to the cart, or increase quantity if already present.
  cart: Cart
  product: Product
  quantity: Int

  requires {
    cart.checked_out == false
    product.status == Available
    quantity > 0
    product.stock >= quantity
  }

  ensures {
    exists item: CartItem =>
      item.product == product &&
      item.quantity == quantity
  }

  properties {
    idempotent: false
  }
}

action RemoveItem {
  --- Remove a product from the cart entirely.
  cart: Cart
  product: Product

  requires {
    cart.checked_out == false
    exists item: CartItem =>
      item.product == product
  }

  ensures {
    !(exists item: CartItem =>
      item.product == product)
  }
}

action UpdateQuantity {
  --- Change the quantity of an item already in the cart.
  cart: Cart
  product: Product
  new_quantity: Int

  requires {
    cart.checked_out == false
    new_quantity > 0
    product.stock >= new_quantity
    exists item: CartItem =>
      item.product == product
  }

  ensures {
    exists item: CartItem =>
      item.product == product &&
      item.quantity == new_quantity
  }
}

action Checkout {
  --- Convert the cart into a confirmed order.
  cart: Cart

  requires {
    cart.checked_out == false
    forall item: CartItem =>
      item.product.stock >= item.quantity
  }

  ensures {
    cart.checked_out == true
    exists o: Order =>
      o.cart == cart &&
      o.status == Confirmed
  }

  properties {
    atomic: true
    audit_logged: true
  }
}

invariant StockNonNegative {
  --- Product stock can never go below zero.
  forall p: Product => p.stock >= 0
}

invariant CartItemsPositive {
  --- Every item in a cart must have a positive quantity.
  forall item: CartItem => item.quantity > 0
}

edge_cases {
  when product.status == Discontinued => reject("Product is no longer available")
  when product.stock < quantity => reject("Insufficient stock")
  when cart.checked_out == true => reject("Cart has already been checked out")
}
```

## Key concepts demonstrated

- **Collection types** (`List<CartItem>`)
- **Nested entity references** (`item.product.stock`)
- **Negated existentials** in postconditions (`!(exists ...)`)
- **Quantifiers in preconditions** (`forall item: CartItem => ...` in Checkout)
- **Multiple related actions** forming a complete workflow (add, remove, update, checkout)
