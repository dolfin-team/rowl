from rowl import parse

code = """
# Comment
# namespace restaurant.menu

import food:
  italian as Italian     # Italian = food.italian
  base:
    Ingredient           # Ingredient = food.base.Ingredient
    Recipe
import ingredients.dairy

ontology Truc

ontology RestaurantMenu:  
  concept MenuItem:
    sub Europe.Italian.Food, Recipe
    has price: one float
    has portions: 1..10 int
    has extras: 0..* Ingredient

  concept Dish:
    sub MenuItem, dairy.DairyProduct
    has name: one string
    has allergens: any string

  property available: MenuItem -> one boolean

"""


def main():
    tree = parse(code)
    # print(tree.pretty())
    print(tree)


if __name__ == "__main__":
    main()
