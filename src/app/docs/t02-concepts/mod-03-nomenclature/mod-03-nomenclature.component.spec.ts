import { ComponentFixture, TestBed } from '@angular/core/testing';

import { Mod03NomenclatureComponent } from './mod-03-nomenclature.component';

describe('Mod03NomenclatureComponent', () => {
  let component: Mod03NomenclatureComponent;
  let fixture: ComponentFixture<Mod03NomenclatureComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [Mod03NomenclatureComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(Mod03NomenclatureComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
