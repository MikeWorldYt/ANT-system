import { ComponentFixture, TestBed } from '@angular/core/testing';

import { Docs_T01_IntoComponent } from './into-title-01.component';

describe('Docs_T01_IntoComponent', () => {
  let component: Docs_T01_IntoComponent;
  let fixture: ComponentFixture<Docs_T01_IntoComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [Docs_T01_IntoComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(Docs_T01_IntoComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
